import { z } from 'zod'
import { request, requestBinary, requestFormData } from '@/api/client'
import { authHeaders } from '@/api/endpoints/auth'
import { ImportJobSchema, type ImportJob } from '@/api/schemas/importJob'

export type ImportLinkInput = {
  url: string
  title?: string
}

export type ImportUploadOptions = {
  signal?: AbortSignal | undefined
  onProgress?: (percent: number) => void
  onJobCreated?: (job: ImportJob) => void
}

const ChunkedImportInitSchema = z.object({
  job: ImportJobSchema,
  chunkSizeBytes: z.number().int().positive(),
})

const ChunkUploadSchema = z.object({
  receivedBytes: z.number().int().nonnegative(),
  totalBytes: z.number().int().nonnegative(),
  progress: z.number().int().min(0).max(100),
})

const useMock = import.meta.env.DEV && import.meta.env['VITE_USE_MOCK'] !== 'false'

export async function createImportJob(
  files: File[],
  links: ImportLinkInput[],
  prompt = '',
  options: ImportUploadOptions = {},
): Promise<ImportJob> {
  if (!files.length || useMock) {
    const body = new FormData()
    files.forEach((file) => body.append('file', file))
    body.append('links', JSON.stringify(links))
    body.append('prompt', prompt)
    return requestFormData({
      method: 'POST',
      path: '/api/v1/import-jobs',
      body,
      schema: ImportJobSchema,
      headers: authHeaders(),
      signal: options.signal,
    })
  }

  options.onProgress?.(0)
  const initialized = await request({
    method: 'POST',
    path: '/api/v1/import-jobs/chunked',
    body: {
      files: files.map((file) => ({
        name: file.name,
        sizeBytes: file.size,
        mimeType: file.type || 'application/octet-stream',
      })),
      links,
      prompt,
    },
    schema: ChunkedImportInitSchema,
    headers: authHeaders(),
    signal: options.signal,
  })
  options.onJobCreated?.(initialized.job)

  const inputs = initialized.job.inputs.filter((input) => input.inputKind === 'file')
  if (inputs.length !== files.length) {
    throw new Error('服务器返回的附件清单与本地文件不一致')
  }
  const totalBytes = files.reduce((total, file) => total + file.size, 0)
  let uploadedBytes = 0
  for (const [fileIndex, file] of files.entries()) {
    const input = inputs[fileIndex]!
    for (let offset = 0; offset < file.size; offset += initialized.chunkSizeBytes) {
      const chunk = file.slice(offset, Math.min(offset + initialized.chunkSizeBytes, file.size))
      await uploadChunkWithRetry(initialized.job.id, input.id, offset, chunk, options.signal)
      uploadedBytes += chunk.size
      options.onProgress?.(
        totalBytes === 0 ? 100 : Math.min(100, Math.round((uploadedBytes / totalBytes) * 100)),
      )
    }
  }

  const completed = await request({
    method: 'POST',
    path: `/api/v1/import-jobs/${encodeURIComponent(initialized.job.id)}/complete`,
    body: {},
    schema: ImportJobSchema,
    headers: authHeaders(),
    signal: options.signal,
  })
  options.onProgress?.(100)
  return completed
}

async function uploadChunkWithRetry(
  jobId: string,
  inputId: string,
  offset: number,
  chunk: Blob,
  signal: AbortSignal | undefined,
): Promise<void> {
  let lastError: unknown
  for (let attempt = 0; attempt < 3; attempt += 1) {
    if (signal?.aborted) throw new DOMException('上传已取消', 'AbortError')
    try {
      await requestBinary({
        method: 'PUT',
        path: `/api/v1/import-jobs/${encodeURIComponent(jobId)}/inputs/${encodeURIComponent(inputId)}/chunks`,
        body: chunk,
        schema: ChunkUploadSchema,
        headers: {
          ...authHeaders(),
          'Content-Type': 'application/octet-stream',
          'X-Upload-Offset': String(offset),
        },
        signal,
      })
      return
    } catch (error) {
      lastError = error
      if (signal?.aborted || attempt === 2) throw error
      await waitForRetry(350 * 2 ** attempt, signal)
    }
  }
  throw lastError
}

function waitForRetry(milliseconds: number, signal: AbortSignal | undefined): Promise<void> {
  return new Promise((resolve, reject) => {
    if (signal?.aborted) {
      reject(new DOMException('上传已取消', 'AbortError'))
      return
    }
    const timer = window.setTimeout(resolve, milliseconds)
    signal?.addEventListener(
      'abort',
      () => {
        window.clearTimeout(timer)
        reject(new DOMException('上传已取消', 'AbortError'))
      },
      { once: true },
    )
  })
}

export function getImportJob(id: string): Promise<ImportJob> {
  return request({
    method: 'GET',
    path: `/api/v1/import-jobs/${encodeURIComponent(id)}`,
    schema: ImportJobSchema,
    headers: authHeaders(),
  })
}

export function cancelImportJob(id: string): Promise<ImportJob> {
  return request({
    method: 'POST',
    path: `/api/v1/import-jobs/${encodeURIComponent(id)}/cancel`,
    schema: ImportJobSchema,
    headers: authHeaders(),
  })
}

export function saveImportRefinement(id: string, prompt: string): Promise<ImportJob> {
  return request({
    method: 'POST',
    path: `/api/v1/import-jobs/${encodeURIComponent(id)}/refine`,
    body: { prompt },
    schema: ImportJobSchema,
    headers: authHeaders(),
  })
}

export function publishImportGitHub(id: string): Promise<ImportJob> {
  return request({
    method: 'POST',
    path: `/api/v1/import-jobs/${encodeURIComponent(id)}/github/publish`,
    schema: ImportJobSchema,
    headers: authHeaders(),
  })
}

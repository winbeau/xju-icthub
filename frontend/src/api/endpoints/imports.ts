import { request, requestFormData } from '@/api/client'
import { authHeaders } from '@/api/endpoints/auth'
import { ImportJobSchema, type ImportJob } from '@/api/schemas/importJob'

export type ImportLinkInput = {
  url: string
  title?: string
}

export function createImportJob(
  files: File[],
  links: ImportLinkInput[],
  prompt = '',
): Promise<ImportJob> {
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

import { useCallback, useEffect, useRef, useState } from 'react'
import {
  Check,
  Copy,
  Download,
  ExternalLink,
  FileQuestion,
  LoaderCircle,
  Maximize2,
  MousePointerClick,
  ZoomIn,
  ZoomOut,
} from 'lucide-react'
import type { PDFDocumentProxy, PDFPageProxy, RenderTask } from 'pdfjs-dist'
import workerUrl from 'pdfjs-dist/build/pdf.worker.min.mjs?url'
import 'highlight.js/styles/github.css'
import { resolveApiUrl } from '@/api/client'
import {
  createProjectResourcePreview,
  loadProjectResource,
} from '@/api/endpoints/projects'
import type { ProjectResource, ResourcePreviewTicket } from '@/api/schemas/project'
import { Button } from '@/components/ui/button'

type Props = {
  slug: string
  resource: ProjectResource | null
  createHtmlPreview?: (() => Promise<ResourcePreviewTicket>) | undefined
}

export function ResourcePreviewPane({ slug, resource, createHtmlPreview }: Props) {
  if (!resource) {
    return (
      <div className="flex h-full flex-col items-center justify-center gap-3 px-8 text-center">
        <MousePointerClick className="size-9 text-text-faint" strokeWidth={1.5} aria-hidden />
        <div>
          <p className="text-sm font-medium">选择一项资料</p>
          <p className="mt-1 text-sm text-text-muted">文档、图片、视频和 HTML 演示会在这里打开。</p>
        </div>
      </div>
    )
  }
  return (
    <Preview
      resource={resource}
      slug={slug}
      createHtmlPreview={createHtmlPreview}
      key={resource.id}
    />
  )
}

function Preview({
  resource,
  slug,
  createHtmlPreview,
}: {
  resource: ProjectResource
  slug: string
  createHtmlPreview?: (() => Promise<ResourcePreviewTicket>) | undefined
}) {
  const download = useDownloadResource(resource)
  const actions = (
    <div className="flex shrink-0 items-center gap-1">
      {resource.url && (
        <Button asChild variant="ghost" size="icon" className="size-8" title="新窗口打开">
          <a href={resource.url} target="_blank" rel="noreferrer" aria-label="新窗口打开">
            <ExternalLink className="size-4" />
          </a>
        </Button>
      )}
      {resource.downloadUrl && (
        <Button
          variant="ghost"
          size="icon"
          className="size-8"
          onClick={download.run}
          disabled={download.loading}
          title="下载文件"
          aria-label="下载文件"
        >
          {download.loading ? (
            <LoaderCircle className="size-4 animate-spin" />
          ) : (
            <Download className="size-4" />
          )}
        </Button>
      )}
    </div>
  )

  if (resource.previewKind === 'html_bundle') {
    return (
      <HtmlPresentationPreview
        resource={resource}
        slug={slug}
        actions={actions}
        createHtmlPreview={createHtmlPreview}
      />
    )
  }
  if (resource.contentUrl && resource.previewKind === 'pdf') {
    return <PdfPreview resource={resource} actions={actions} />
  }
  if (resource.contentUrl && resource.previewKind === 'docx') {
    return <DocxPreview resource={resource} actions={actions} />
  }
  if (resource.contentUrl && resource.previewKind === 'image') {
    return <ImagePreview resource={resource} actions={actions} />
  }
  if (resource.contentUrl && resource.previewKind === 'video') {
    return <VideoPreview resource={resource} actions={actions} />
  }
  if (resource.contentUrl && resource.previewKind === 'code') {
    return <CodePreview resource={resource} actions={actions} />
  }
  return <UnsupportedPreview resource={resource} actions={actions} download={download.run} />
}

function PreviewHeader({
  resource,
  actions,
  tools,
}: {
  resource: ProjectResource
  actions: React.ReactNode
  tools?: React.ReactNode
}) {
  return (
    <div className="flex h-12 shrink-0 items-center gap-2 border-b border-border bg-bg px-4">
      <span className="min-w-0 flex-1 truncate text-sm font-medium" title={resource.title}>
        {resource.title}
      </span>
      {resource.sizeBytes != null && (
        <span className="hidden text-xs text-text-faint lg:inline">
          {formatBytes(resource.sizeBytes)}
        </span>
      )}
      {tools}
      {actions}
    </div>
  )
}

function PdfPreview({ resource, actions }: { resource: ProjectResource; actions: React.ReactNode }) {
  const [document, setDocument] = useState<PDFDocumentProxy | null>(null)
  const [error, setError] = useState('')
  const [zoom, setZoom] = useState(1)
  const contentUrl = resource.contentUrl!

  useEffect(() => {
    const abort = new AbortController()
    let opened: PDFDocumentProxy | null = null
    setDocument(null)
    setError('')
    loadProjectResource(contentUrl, abort.signal)
      .then((blob) => blob.arrayBuffer())
      .then(async (buffer) => {
        const pdfjs = await import('pdfjs-dist')
        pdfjs.GlobalWorkerOptions.workerSrc = workerUrl
        return pdfjs.getDocument({ data: buffer }).promise
      })
      .then((pdf) => {
        opened = pdf
        if (!abort.signal.aborted) setDocument(pdf)
      })
      .catch((caught: unknown) => {
        if (!abort.signal.aborted) setError(errorMessage(caught))
      })
    return () => {
      abort.abort()
      void opened?.destroy()
    }
  }, [contentUrl])

  const tools = (
    <ZoomTools zoom={zoom} onZoom={setZoom} onFit={() => setZoom(1)} />
  )
  return (
    <div className="flex h-full min-h-0 flex-col">
      <PreviewHeader resource={resource} actions={actions} tools={tools} />
      <div className="min-h-0 flex-1 overflow-auto bg-bg-subtle p-4">
        {error ? (
          <PreviewError message={error} />
        ) : document ? (
          <div className="mx-auto flex w-fit min-w-full flex-col items-center gap-4">
            {Array.from({ length: document.numPages }, (_, index) => (
              <PdfPage key={index + 1} document={document} pageNumber={index + 1} zoom={zoom} />
            ))}
          </div>
        ) : (
          <PreviewLoading label="正在打开 PDF" />
        )}
      </div>
    </div>
  )
}

function PdfPage({
  document,
  pageNumber,
  zoom,
}: {
  document: PDFDocumentProxy
  pageNumber: number
  zoom: number
}) {
  const hostRef = useRef<HTMLDivElement>(null)
  const canvasRef = useRef<HTMLCanvasElement>(null)
  const [page, setPage] = useState<PDFPageProxy | null>(null)
  const [visible, setVisible] = useState(false)
  const [size, setSize] = useState({ width: 640, height: 900 })
  const [error, setError] = useState('')

  useEffect(() => {
    const host = hostRef.current
    if (!host) return
    const observer = new IntersectionObserver(
      (entries) => {
        if (entries.some((entry) => entry.isIntersecting)) {
          setVisible(true)
          observer.disconnect()
        }
      },
      { rootMargin: '400px 0px' },
    )
    observer.observe(host)
    return () => observer.disconnect()
  }, [])

  useEffect(() => {
    let cancelled = false
    document
      .getPage(pageNumber)
      .then((loadedPage) => {
        if (cancelled) return
        const viewport = loadedPage.getViewport({ scale: zoom })
        setPage(loadedPage)
        setSize({ width: viewport.width, height: viewport.height })
      })
      .catch((caught: unknown) => {
        if (!cancelled) setError(errorMessage(caught))
      })
    return () => {
      cancelled = true
    }
  }, [document, pageNumber, zoom])

  useEffect(() => {
    if (!visible || !page || !canvasRef.current) return
    let cancelled = false
    let task: RenderTask | null = null
    const viewport = page.getViewport({ scale: zoom })
    const ratio = window.devicePixelRatio || 1
    const canvas = canvasRef.current
    const context = canvas.getContext('2d')
    if (!context) return
    canvas.width = Math.floor(viewport.width * ratio)
    canvas.height = Math.floor(viewport.height * ratio)
    canvas.style.width = `${Math.floor(viewport.width)}px`
    canvas.style.height = `${Math.floor(viewport.height)}px`
    task = page.render({
      canvas,
      canvasContext: context,
      viewport,
      transform: ratio === 1 ? undefined : [ratio, 0, 0, ratio, 0, 0],
    })
    task.promise.catch((caught: unknown) => {
      if (!cancelled && (caught as { name?: string }).name !== 'RenderingCancelledException') {
        setError(errorMessage(caught))
      }
    })
    return () => {
      cancelled = true
      task?.cancel()
    }
  }, [page, visible, zoom])

  if (error) return <PreviewError message={`第 ${pageNumber} 页渲染失败：${error}`} />
  return (
    <div
      ref={hostRef}
      className="rounded-sm bg-white shadow-card"
      style={{ width: size.width, height: size.height }}
      aria-label={`第 ${pageNumber} 页`}
    >
      {visible && <canvas ref={canvasRef} className="block" />}
    </div>
  )
}

function DocxPreview({ resource, actions }: { resource: ProjectResource; actions: React.ReactNode }) {
  const hostRef = useRef<HTMLDivElement>(null)
  const [zoom, setZoom] = useState(1)
  const [loading, setLoading] = useState(true)
  const [error, setError] = useState('')
  const contentUrl = resource.contentUrl!
  useEffect(() => {
    const abort = new AbortController()
    const host = hostRef.current
    if (!host) return
    host.replaceChildren()
    setLoading(true)
    setError('')
    loadProjectResource(contentUrl, abort.signal)
      .then(async (blob) => {
        const { renderAsync } = await import('docx-preview')
        if (abort.signal.aborted) return
        await renderAsync(blob, host, undefined, {
          className: 'docx',
          inWrapper: true,
          breakPages: true,
          experimental: true,
          useBase64URL: true,
        })
        if (!abort.signal.aborted) setLoading(false)
      })
      .catch((caught: unknown) => {
        if (!abort.signal.aborted) {
          setLoading(false)
          setError(errorMessage(caught))
        }
      })
    return () => {
      abort.abort()
      host.replaceChildren()
    }
  }, [contentUrl])
  return (
    <div className="flex h-full min-h-0 flex-col">
      <PreviewHeader
        resource={resource}
        actions={actions}
        tools={<ZoomTools zoom={zoom} onZoom={setZoom} onFit={() => setZoom(1)} />}
      />
      <div className="min-h-0 flex-1 overflow-auto bg-bg-subtle p-4">
        {error && <PreviewError message={error} />}
        {loading && <PreviewLoading label="正在排版 Word 文档" />}
        <div className="mx-auto w-max min-w-full">
          <div
            ref={hostRef}
            className="ict-docx-preview"
            style={{ zoom }}
          />
        </div>
      </div>
    </div>
  )
}

function ImagePreview({ resource, actions }: { resource: ProjectResource; actions: React.ReactNode }) {
  const blob = useResourceUrl(resource.contentUrl!)
  return (
    <div className="flex h-full min-h-0 flex-col">
      <PreviewHeader resource={resource} actions={actions} />
      <div className="flex min-h-0 flex-1 items-center justify-center overflow-auto bg-bg-subtle p-5">
        {blob.error ? (
          <PreviewError message={blob.error} />
        ) : blob.url ? (
          <img src={blob.url} alt={resource.title} className="max-h-full max-w-full object-contain" />
        ) : (
          <PreviewLoading label="正在加载图片" />
        )}
      </div>
    </div>
  )
}

function VideoPreview({ resource, actions }: { resource: ProjectResource; actions: React.ReactNode }) {
  const blob = useResourceUrl(resource.contentUrl!)
  return (
    <div className="flex h-full min-h-0 flex-col">
      <PreviewHeader resource={resource} actions={actions} />
      <div className="flex min-h-0 flex-1 items-center justify-center bg-neutral-950 p-4">
        {blob.error ? (
          <PreviewError message={blob.error} />
        ) : blob.url ? (
          <video src={blob.url} controls className="max-h-full max-w-full" preload="metadata" />
        ) : (
          <PreviewLoading label="正在加载视频" />
        )}
      </div>
    </div>
  )
}

function CodePreview({ resource, actions }: { resource: ProjectResource; actions: React.ReactNode }) {
  const [html, setHtml] = useState('')
  const [raw, setRaw] = useState('')
  const [error, setError] = useState('')
  const [copied, setCopied] = useState(false)
  const contentUrl = resource.contentUrl!
  useEffect(() => {
    const abort = new AbortController()
    loadProjectResource(contentUrl, abort.signal)
      .then((blob) => blob.text())
      .then(async (text) => {
        const preview = text.slice(0, 500_000)
        const highlight = (await import('highlight.js/lib/common')).default
        if (abort.signal.aborted) return
        setRaw(preview)
        setHtml(highlight.highlightAuto(preview).value)
      })
      .catch((caught: unknown) => {
        if (!abort.signal.aborted) setError(errorMessage(caught))
      })
    return () => abort.abort()
  }, [contentUrl])
  const copy = () => {
    void navigator.clipboard.writeText(raw).then(() => {
      setCopied(true)
      window.setTimeout(() => setCopied(false), 1200)
    })
  }
  return (
    <div className="flex h-full min-h-0 flex-col">
      <PreviewHeader
        resource={resource}
        actions={actions}
        tools={
          <Button variant="ghost" size="sm" className="h-8 gap-1.5 text-xs" onClick={copy}>
            {copied ? <Check className="size-3.5" /> : <Copy className="size-3.5" />}
            {copied ? '已复制' : '复制'}
          </Button>
        }
      />
      <div className="min-h-0 flex-1 overflow-auto bg-bg-subtle">
        {error ? (
          <PreviewError message={error} />
        ) : html ? (
          <pre className="hljs m-0 min-h-full p-5 text-xs leading-relaxed">
            <code dangerouslySetInnerHTML={{ __html: html }} />
          </pre>
        ) : (
          <PreviewLoading label="正在读取文本" />
        )}
      </div>
    </div>
  )
}

function HtmlPresentationPreview({
  resource,
  slug,
  actions,
  createHtmlPreview,
}: {
  resource: ProjectResource
  slug: string
  actions: React.ReactNode
  createHtmlPreview?: (() => Promise<ResourcePreviewTicket>) | undefined
}) {
  const [url, setUrl] = useState('')
  const [error, setError] = useState('')
  useEffect(() => {
    let cancelled = false
    ;(createHtmlPreview ?? (() => createProjectResourcePreview(slug, resource.id)))()
      .then((ticket) => {
        if (!cancelled) setUrl(resolveApiUrl(ticket.url))
      })
      .catch((caught: unknown) => {
        if (!cancelled) setError(errorMessage(caught))
      })
    return () => {
      cancelled = true
    }
  }, [createHtmlPreview, resource.id, slug])
  return (
    <div className="flex h-full min-h-0 flex-col">
      <PreviewHeader resource={resource} actions={actions} />
      <div className="min-h-0 flex-1 bg-neutral-950">
        {error ? (
          <PreviewError message={error} />
        ) : url ? (
          <iframe
            src={url}
            title={resource.title}
            className="h-full w-full border-0 bg-white"
            sandbox="allow-scripts allow-forms allow-pointer-lock allow-presentation"
            allow="fullscreen"
            referrerPolicy="no-referrer"
          />
        ) : (
          <PreviewLoading label="正在建立安全演示窗口" />
        )}
      </div>
    </div>
  )
}

function UnsupportedPreview({
  resource,
  actions,
  download,
}: {
  resource: ProjectResource
  actions: React.ReactNode
  download: () => void
}) {
  return (
    <div className="flex h-full min-h-0 flex-col">
      <PreviewHeader resource={resource} actions={actions} />
      <div className="flex min-h-0 flex-1 flex-col items-center justify-center gap-4 p-8 text-center">
        <FileQuestion className="size-12 text-text-faint" strokeWidth={1.35} aria-hidden />
        <div>
          <p className="font-medium">{resource.title}</p>
          <p className="mt-1 text-sm text-text-muted">
            {resource.url ? '这是外部资料，请在新窗口打开。' : '该格式暂不支持在线预览，可下载后查看。'}
          </p>
        </div>
        {resource.downloadUrl && (
          <Button variant="outline" onClick={download}>
            <Download aria-hidden />下载文件
          </Button>
        )}
      </div>
    </div>
  )
}

function ZoomTools({
  zoom,
  onZoom,
  onFit,
}: {
  zoom: number
  onZoom: React.Dispatch<React.SetStateAction<number>>
  onFit: () => void
}) {
  const adjust = (delta: number) => onZoom((value) => Math.min(2.5, Math.max(0.4, value + delta)))
  return (
    <div className="flex shrink-0 items-center gap-0.5">
      <Button variant="ghost" size="icon" className="size-8" onClick={() => adjust(-0.1)} aria-label="缩小">
        <ZoomOut className="size-4" />
      </Button>
      <span className="w-11 text-center text-xs tabular-nums text-text-muted">
        {Math.round(zoom * 100)}%
      </span>
      <Button variant="ghost" size="icon" className="size-8" onClick={() => adjust(0.1)} aria-label="放大">
        <ZoomIn className="size-4" />
      </Button>
      <Button variant="ghost" size="icon" className="size-8" onClick={onFit} aria-label="适应宽度">
        <Maximize2 className="size-4" />
      </Button>
    </div>
  )
}

function PreviewLoading({ label }: { label: string }) {
  return (
    <div className="flex h-full min-h-48 items-center justify-center gap-2 text-sm text-text-muted">
      <LoaderCircle className="size-5 animate-spin" aria-hidden />
      {label}
    </div>
  )
}

function PreviewError({ message }: { message: string }) {
  return (
    <div className="flex h-full min-h-48 items-center justify-center px-8 text-center text-sm text-cat-internet">
      {message}
    </div>
  )
}

function useResourceUrl(path: string) {
  const [url, setUrl] = useState('')
  const [error, setError] = useState('')
  useEffect(() => {
    const abort = new AbortController()
    let objectUrl = ''
    loadProjectResource(path, abort.signal)
      .then((blob) => {
        objectUrl = URL.createObjectURL(blob)
        setUrl(objectUrl)
      })
      .catch((caught: unknown) => {
        if (!abort.signal.aborted) setError(errorMessage(caught))
      })
    return () => {
      abort.abort()
      if (objectUrl) URL.revokeObjectURL(objectUrl)
    }
  }, [path])
  return { url, error }
}

function useDownloadResource(resource: ProjectResource) {
  const [loading, setLoading] = useState(false)
  const run = useCallback(() => {
    if (!resource.downloadUrl || loading) return
    setLoading(true)
    loadProjectResource(resource.downloadUrl)
      .then((blob) => {
        const url = URL.createObjectURL(blob)
        const anchor = document.createElement('a')
        anchor.href = url
        anchor.download = resource.sourceName || resource.title
        anchor.click()
        window.setTimeout(() => URL.revokeObjectURL(url), 1_000)
      })
      .finally(() => setLoading(false))
  }, [loading, resource.downloadUrl, resource.sourceName, resource.title])
  return { loading, run }
}

function formatBytes(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`
  return `${(bytes / 1024 / 1024).toFixed(1)} MB`
}

function errorMessage(error: unknown): string {
  return error instanceof Error ? error.message : '预览加载失败，请下载后查看。'
}

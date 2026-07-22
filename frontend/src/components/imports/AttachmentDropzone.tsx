import { useRef, useState } from 'react'
import {
  Archive,
  FileCode2,
  FileQuestion,
  FileText,
  Film,
  Image as ImageIcon,
  Presentation,
  UploadCloud,
  X,
} from 'lucide-react'
import { Button } from '@/components/ui/button'

const MAX_FILE_BYTES = 256 * 1024 * 1024

export function AttachmentDropzone({
  files,
  onChange,
}: {
  files: File[]
  onChange: (files: File[]) => void
}) {
  const inputRef = useRef<HTMLInputElement>(null)
  const dragCounter = useRef(0)
  const [dragging, setDragging] = useState(false)

  const addFiles = (incoming: FileList | File[]) => {
    const accepted: File[] = []
    for (const file of Array.from(incoming)) {
      if (file.size > MAX_FILE_BYTES) continue
      if (!files.some((current) => fileKey(current) === fileKey(file)) && !accepted.some((current) => fileKey(current) === fileKey(file))) {
        accepted.push(file)
      }
    }
    if (accepted.length) onChange([...files, ...accepted])
  }

  return (
    <div>
      <div
        role="button"
        tabIndex={0}
        className={`rounded-xl border border-dashed px-5 py-7 transition-colors ${dragging ? 'border-text bg-bg-subtle' : 'border-border-strong hover:bg-bg-subtle'}`}
        onClick={() => inputRef.current?.click()}
        onKeyDown={(event) => {
          if (event.key === 'Enter' || event.key === ' ') inputRef.current?.click()
        }}
        onDragEnter={(event) => {
          event.preventDefault()
          dragCounter.current += 1
          if (event.dataTransfer.types.includes('Files')) setDragging(true)
        }}
        onDragOver={(event) => event.preventDefault()}
        onDragLeave={(event) => {
          event.preventDefault()
          dragCounter.current = Math.max(0, dragCounter.current - 1)
          if (!dragCounter.current) setDragging(false)
        }}
        onDrop={(event) => {
          event.preventDefault()
          dragCounter.current = 0
          setDragging(false)
          addFiles(event.dataTransfer.files)
        }}
      >
        <input
          ref={inputRef}
          type="file"
          multiple
          className="sr-only"
          onChange={(event) => {
            if (event.target.files) addFiles(event.target.files)
            event.currentTarget.value = ''
          }}
        />
        <div className="flex items-center gap-4">
          <UploadCloud className="size-7 shrink-0 text-text-muted" aria-hidden />
          <div>
            <p className="font-medium">拖入附件，或点击选择</p>
            <p className="mt-1 text-sm leading-6 text-text-muted">
              源码、文档、PPT、图片、视频和压缩包都可以放在一起
            </p>
          </div>
        </div>
      </div>

      {files.length > 0 && (
        <div className="mt-3 divide-y divide-border rounded-lg border border-border">
          {files.map((file) => (
            <div key={fileKey(file)} className="flex items-center gap-3 px-3 py-2.5">
              <FileIcon file={file} />
              <div className="min-w-0 flex-1">
                <p className="truncate text-sm">{file.name}</p>
                <p className="mt-0.5 text-xs text-text-faint">
                  {kindLabel(file)} · {formatBytes(file.size)}
                </p>
              </div>
              <Button
                type="button"
                variant="ghost"
                size="icon"
                aria-label={`移除 ${file.name}`}
                onClick={(event) => {
                  event.stopPropagation()
                  onChange(files.filter((current) => fileKey(current) !== fileKey(file)))
                }}
              >
                <X aria-hidden />
              </Button>
            </div>
          ))}
        </div>
      )}
    </div>
  )
}

function FileIcon({ file }: { file: File }) {
  const className = 'size-4 shrink-0 text-text-muted'
  switch (kindLabel(file)) {
    case '源码': return <FileCode2 className={className} aria-hidden />
    case '文档': return <FileText className={className} aria-hidden />
    case 'PPT': return <Presentation className={className} aria-hidden />
    case '视频': return <Film className={className} aria-hidden />
    case '图片': return <ImageIcon className={className} aria-hidden />
    case '压缩包': return <Archive className={className} aria-hidden />
    default: return <FileQuestion className={className} aria-hidden />
  }
}

function kindLabel(file: File): string {
  const extension = file.name.split('.').pop()?.toLowerCase() ?? ''
  if (['rs', 'ts', 'tsx', 'js', 'jsx', 'py', 'java', 'go', 'c', 'cpp', 'h', 'hpp', 'cs', 'php', 'rb', 'swift', 'dart', 'vue', 'svelte', 'html', 'css', 'scss', 'sql', 'sh', 'ps1', 'toml', 'yaml', 'yml', 'json'].includes(extension)) return '源码'
  if (['pdf', 'doc', 'docx', 'odt', 'rtf', 'md', 'txt', 'tex'].includes(extension)) return '文档'
  if (['ppt', 'pptx', 'key', 'odp'].includes(extension)) return 'PPT'
  if (['mp4', 'mov', 'mkv', 'avi', 'webm', 'm4v', 'wmv'].includes(extension)) return '视频'
  if (['png', 'jpg', 'jpeg', 'gif', 'webp', 'svg', 'bmp', 'tiff'].includes(extension)) return '图片'
  if (['zip', 'rar', '7z', 'tar', 'gz', 'bz2', 'xz'].includes(extension)) return '压缩包'
  return '其他'
}

function fileKey(file: File): string {
  return `${file.name}:${file.size}:${file.lastModified}`
}

function formatBytes(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`
  if (bytes < 1024 * 1024 * 1024) return `${(bytes / 1024 / 1024).toFixed(1)} MB`
  return `${(bytes / 1024 / 1024 / 1024).toFixed(1)} GB`
}

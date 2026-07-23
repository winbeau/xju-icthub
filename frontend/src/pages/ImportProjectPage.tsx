import { useMemo, useRef, useState } from 'react'
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import {
  Archive,
  ArrowLeft,
  ArrowRight,
  FileCode2,
  FileQuestion,
  FileText,
  Film,
  Github,
  Image as ImageIcon,
  Link2,
  LoaderCircle,
  PackageOpen,
  Presentation,
  ShieldCheck,
  RotateCcw,
  Workflow,
} from 'lucide-react'
import { Link, useNavigate, useSearchParams } from 'react-router-dom'
import { toast } from 'sonner'
import {
  cancelImportJob,
  createImportJob,
  getImportJob,
  publishImportGitHub,
  saveImportRefinement,
  type ImportLinkInput,
} from '@/api/endpoints/imports'
import type { ImportArtifact, ImportJob } from '@/api/schemas/importJob'
import type { ProjectResourceInput, ProjectWriteInput } from '@/api/schemas/project'
import { AttachmentDropzone } from '@/components/imports/AttachmentDropzone'
import { agentStatusLabel } from '@/components/imports/importWorkflowState'
import { ImportWorkflowDialog } from '@/components/imports/ImportWorkflowDialog'
import { ImportWorkflowProgress } from '@/components/imports/ImportWorkflowProgress'
import { CreateModeSwitch } from '@/components/projects/CreateModeSwitch'
import { Button } from '@/components/ui/button'
import { Textarea } from '@/components/ui/textarea'

const TERMINAL_STATUSES = new Set(['completed', 'failed', 'cancelled'])

export function ImportProjectPage() {
  const navigate = useNavigate()
  const [searchParams, setSearchParams] = useSearchParams()
  const queryClient = useQueryClient()
  const [prompt, setPrompt] = useState('')
  const [linkText, setLinkText] = useState('')
  const [files, setFiles] = useState<File[]>([])
  const [additionalPrompt, setAdditionalPrompt] = useState('')
  const [jobId, setJobId] = useState<string | null>(() => searchParams.get('job'))
  const [workflowOpen, setWorkflowOpen] = useState(false)
  const [draftSaved, setDraftSaved] = useState(false)
  const [uploadProgress, setUploadProgress] = useState(0)
  const uploadAbortRef = useRef<AbortController | null>(null)
  const links = useMemo(() => parseLinks(linkText), [linkText])
  const readyToSubmit = Boolean(prompt.trim() || links.length || files.length)

  const upload = useMutation({
    mutationFn: () => {
      if (!readyToSubmit) throw new Error('请至少填写简介、链接或上传一个附件')
      const controller = new AbortController()
      uploadAbortRef.current = controller
      setUploadProgress(0)
      return createImportJob(files, links, prompt, {
        signal: controller.signal,
        onProgress: setUploadProgress,
        onJobCreated: (created) => {
          setJobId(created.id)
          setSearchParams({ job: created.id }, { replace: true })
          queryClient.setQueryData(['import-job', created.id], created)
          setWorkflowOpen(true)
        },
      })
    },
    onSuccess: (job) => {
      setJobId(job.id)
      setSearchParams({ job: job.id }, { replace: true })
      queryClient.setQueryData(['import-job', job.id], job)
      setWorkflowOpen(true)
      toast.success('导入任务已启动')
    },
    onError: (error) => {
      if (error instanceof DOMException && error.name === 'AbortError') return
      toast.error(error instanceof Error ? error.message : '上传失败')
    },
    onSettled: () => {
      uploadAbortRef.current = null
    },
  })
  const jobQuery = useQuery({
    queryKey: ['import-job', jobId],
    queryFn: () => getImportJob(jobId!),
    enabled: Boolean(jobId),
    refetchInterval: (query) => {
      const status = query.state.data?.status
      const publicationStatus = query.state.data?.githubPublication?.status
      const publishing = publicationStatus === 'queued' || publicationStatus === 'running'
      return status && TERMINAL_STATUSES.has(status) && !publishing ? false : 900
    },
  })
  const job = jobQuery.data ?? upload.data
  const busy = upload.isPending || Boolean(job && !TERMINAL_STATUSES.has(job.status))

  const cancel = useMutation({
    mutationFn: (id: string) => cancelImportJob(id),
    onSuccess: () => {
      setJobId(null)
      setSearchParams({}, { replace: true })
      upload.reset()
      setUploadProgress(0)
      setWorkflowOpen(false)
      setDraftSaved(true)
      toast.success('整理已取消，当前填写已保留')
    },
    onError: (error) => toast.error(error instanceof Error ? error.message : '取消失败'),
  })
  const refine = useMutation({
    mutationFn: ({ id, value }: { id: string; value: string }) => saveImportRefinement(id, value),
    onSuccess: (saved) => {
      queryClient.setQueryData(['import-job', saved.id], saved)
      toast.success(
        saved.status === 'agent_queued'
          ? '补充提示已保存，Codex 已排队'
          : '补充提示已保存，等待 Codex 配置',
      )
    },
    onError: (error) => toast.error(error instanceof Error ? error.message : '补充提示保存失败'),
  })
  const publishGitHub = useMutation({
    mutationFn: (id: string) => publishImportGitHub(id),
    onSuccess: (saved) => {
      queryClient.setQueryData(['import-job', saved.id], saved)
      toast.success(
        saved.githubPublication?.status === 'completed'
          ? '私有源码仓库已创建'
          : '私有源码仓库已进入发布队列',
      )
    },
    onError: (error) =>
      toast.error(error instanceof Error ? error.message : '私有源码仓库发布失败'),
  })

  const reset = () => {
    uploadAbortRef.current?.abort()
    setPrompt('')
    setLinkText('')
    setFiles([])
    setAdditionalPrompt('')
    setJobId(null)
    setSearchParams({}, { replace: true })
    setDraftSaved(false)
    setUploadProgress(0)
    upload.reset()
  }
  const openDraft = () => {
    if (!job?.result) return
    navigate('/admin/projects/new', { state: { importDraft: editorDraft(job) } })
  }

  return (
    <div className="mx-auto max-w-5xl px-5 py-12 sm:px-8 sm:py-16">
      <Button asChild variant="ghost" size="sm" className="-ml-3">
        <Link to="/admin/projects">
          <ArrowLeft aria-hidden />
          返回项目管理
        </Link>
      </Button>
      <header className="mt-6 flex flex-col gap-6 border-b border-border pb-7 sm:flex-row sm:items-end sm:justify-between">
        <div>
          <p className="text-sm font-medium uppercase tracking-[0.14em] text-text-faint">
            Codex Import
          </p>
          <h1 className="mt-3 font-serif text-4xl font-semibold tracking-[-0.025em]">上传项目</h1>
        </div>
        <div className="flex flex-wrap items-center gap-3">
          <Button type="button" variant="outline" onClick={() => setWorkflowOpen(true)}>
            <Workflow aria-hidden />
            工作流程
          </Button>
          {job && (
            <Button variant="ghost" onClick={reset}>
              <RotateCcw aria-hidden />
              重新开始
            </Button>
          )}
          <CreateModeSwitch mode="codex" />
        </div>
      </header>

      {!job && (
        <div className="pt-8">
          {draftSaved && (
            <div className="mb-6 rounded-lg border border-emerald-200 bg-emerald-50/60 px-4 py-3 text-sm text-emerald-800">
              上次整理已取消，简介、链接、附件和补充提示已保留，可以继续修改后重新开始。
            </div>
          )}
          <div className="space-y-9">
            <section>
              <SectionHeading
                index="01"
                title="项目简介"
                description="写下已知背景、项目用途或整理要求；后续会直接作为给 Codex 的提示。"
              />
              <Textarea
                value={prompt}
                onChange={(event) => setPrompt(event.target.value)}
                placeholder="例如：这是一个参加计算机设计大赛的校园工具，请优先识别项目名称、技术栈、文档和展示材料……"
                className="mt-4 min-h-32 text-base leading-7"
              />
            </section>

            <section>
              <SectionHeading
                index="02"
                title="项目链接"
                description="GitHub、网盘、在线文档或视频可以混在一个框里，一行一个或直接整段粘贴。"
              />
              <div className="relative mt-4">
                <Link2
                  className="pointer-events-none absolute left-3 top-3.5 size-4 text-text-faint"
                  aria-hidden
                />
                <Textarea
                  value={linkText}
                  onChange={(event) => setLinkText(event.target.value)}
                  placeholder={
                    'https://github.com/...\nhttps://pan.baidu.com/...\nhttps://www.bilibili.com/...'
                  }
                  className="min-h-28 pl-10 font-mono text-sm leading-7"
                />
              </div>
              {links.length > 0 && (
                <div className="mt-3 flex flex-wrap gap-2">
                  {links.map((link) => (
                    <span
                      key={link.url}
                      className="inline-flex max-w-full items-center gap-2 rounded-full bg-bg-subtle px-3 py-1.5 text-sm text-text-muted"
                    >
                      {link.url.includes('github.com') ? (
                        <Github className="size-3.5 shrink-0" aria-hidden />
                      ) : (
                        <Link2 className="size-3.5 shrink-0" aria-hidden />
                      )}
                      <span className="truncate">{shortLink(link.url)}</span>
                    </span>
                  ))}
                </div>
              )}
            </section>

            <section>
              <SectionHeading
                index="03"
                title="项目附件"
                description="把现有材料一次拖进来；界面沿用飞跃项目的多文件拖放方式。"
              />
              <div className="mt-4">
                <AttachmentDropzone
                  files={files}
                  onChange={(next) => {
                    setFiles(next)
                    setJobId(null)
                    upload.reset()
                  }}
                />
              </div>
            </section>

            <div className="flex flex-col gap-3 border-t border-border pt-6 sm:flex-row sm:items-center sm:justify-between">
              <div className="text-sm leading-6 text-text-muted">
                <p>不会执行附件中的源码或脚本，草稿确认后才会创建项目。</p>
              </div>
              <Button
                size="lg"
                disabled={!readyToSubmit || busy}
                onClick={() => {
                  setDraftSaved(false)
                  setWorkflowOpen(true)
                  upload.mutate()
                }}
              >
                {upload.isPending ? (
                  <LoaderCircle className="animate-spin" aria-hidden />
                ) : (
                  <PackageOpen aria-hidden />
                )}
                {upload.isPending ? '正在上传…' : '开始整理'}
              </Button>
            </div>
          </div>
        </div>
      )}

      {job && (
        <JobResult
          job={job}
          loading={busy || jobQuery.isFetching}
          onOpenDraft={openDraft}
          onPublishGitHub={() => publishGitHub.mutate(job.id)}
          publishingGitHub={publishGitHub.isPending}
        />
      )}
      <ImportWorkflowDialog
        additionalPrompt={additionalPrompt}
        cancelling={cancel.isPending}
        fileCount={files.length}
        job={job}
        linkCount={links.length}
        onAdditionalPromptChange={setAdditionalPrompt}
        onCancel={
          job && !TERMINAL_STATUSES.has(job.status)
            ? () => {
                uploadAbortRef.current?.abort()
                cancel.mutate(job.id)
              }
            : undefined
        }
        onOpenChange={setWorkflowOpen}
        onSavePrompt={
          job ? () => refine.mutate({ id: job.id, value: additionalPrompt }) : undefined
        }
        open={workflowOpen}
        promptProvided={Boolean(prompt.trim())}
        savingPrompt={refine.isPending}
        submitting={upload.isPending}
        uploadProgress={uploadProgress}
      />
    </div>
  )
}

function JobResult({
  job,
  loading,
  onOpenDraft,
  onPublishGitHub,
  publishingGitHub,
}: {
  job: ImportJob
  loading: boolean
  onOpenDraft: () => void
  onPublishGitHub: () => void
  publishingGitHub: boolean
}) {
  const result = job.result
  const groupedArtifacts = useMemo(() => groupArtifacts(job.artifacts), [job.artifacts])
  const failed = job.status === 'failed'
  const publication = job.githubPublication
  const hasSourceCode = result?.normalizedResources.sourceCode.length
  const publishReady = result?.capabilities.githubPublish === 'ready'
  return (
    <div className="pt-8">
      <section className="rounded-xl border border-border p-5 sm:p-6">
        <ImportWorkflowProgress job={job} showEvents={false} />
        {failed && (
          <p className="mt-4 text-sm text-cat-internet">
            {job.errorMessage ?? '解析失败，请重新尝试。'}
          </p>
        )}
      </section>

      {result && (
        <div className="mt-8 grid gap-8 lg:grid-cols-[minmax(0,1fr)_300px]">
          <div className="space-y-8">
            <section>
              <div className="flex items-start justify-between gap-4 border-b border-border pb-4">
                <div>
                  <p className="text-sm text-text-faint">项目草稿</p>
                  <h2 className="mt-2 font-serif text-3xl font-semibold">
                    {result.projectDraft.name}
                  </h2>
                  <p className="mt-3 max-w-2xl text-base leading-7 text-text-muted">
                    {result.projectDraft.summary}
                  </p>
                </div>
                <span className="shrink-0 rounded-full bg-bg-subtle px-3 py-1.5 text-sm">
                  {result.projectDraft.primaryCategory}
                </span>
              </div>
              <div className="mt-4 flex flex-wrap gap-2">
                {result.projectDraft.suggestedTags.map((tag) => (
                  <span
                    key={tag}
                    className="rounded-full border border-border px-3 py-1 text-sm text-text-muted"
                  >
                    {tag}
                  </span>
                ))}
                {!result.projectDraft.suggestedTags.length && (
                  <span className="text-sm text-text-faint">简介未明确指定标签</span>
                )}
              </div>
              <div className="mt-6 flex justify-end">
                <Button onClick={onOpenDraft}>
                  进入人工确认
                  <ArrowRight aria-hidden />
                </Button>
              </div>
            </section>

            <section>
              <SectionHeading
                index="03"
                title="材料清单"
                description={`当前展示 ${job.artifacts.length} 个已索引文件。`}
              />
              <div className="mt-4 divide-y divide-border border-y border-border">
                {Object.entries(groupedArtifacts).map(([kind, artifacts]) => (
                  <details
                    key={kind}
                    open={['presentation', 'document', 'video'].includes(kind)}
                    className="group py-3"
                  >
                    <summary className="flex cursor-pointer list-none items-center gap-3">
                      <ArtifactIcon kind={kind as ImportArtifact['artifactKind']} />
                      <span className="font-medium">{kindLabel(kind)}</span>
                      <span className="text-sm text-text-faint">{artifacts.length}</span>
                    </summary>
                    <ul className="ml-7 mt-3 space-y-2">
                      {artifacts.slice(0, 30).map((artifact) => (
                        <li key={artifact.id} className="flex items-center gap-3 text-sm">
                          <span className="min-w-0 flex-1 truncate text-text-muted">
                            {artifact.relativePath}
                          </span>
                          {artifact.isCoverCandidate && (
                            <span className="rounded bg-bg-subtle px-2 py-0.5 text-xs text-text-faint">
                              封面候选
                            </span>
                          )}
                          <span className="shrink-0 font-mono text-xs text-text-faint">
                            {formatBytes(artifact.sizeBytes)}
                          </span>
                        </li>
                      ))}
                      {artifacts.length > 30 && (
                        <li className="text-sm text-text-faint">
                          另有 {artifacts.length - 30} 个文件
                        </li>
                      )}
                    </ul>
                  </details>
                ))}
              </div>
            </section>
          </div>

          <aside className="space-y-5">
            <div className="rounded-xl bg-bg-subtle p-5">
              <p className="font-medium">解析器状态</p>
              <p className="mt-2 text-sm leading-6 text-text-muted">{result.agent.message}</p>
              <dl className="mt-4 space-y-2 border-t border-border pt-4 text-sm">
                <StatusRow label="多附件收集" value="已打通" />
                <StatusRow label="Codex 分析" value={agentStatusLabel(job)} />
                <StatusRow label="GitHub 链接" value="已预留" />
                <StatusRow
                  label="私有仓库发布"
                  value={githubPublicationStatus(publication?.status, publishReady)}
                />
              </dl>
            </div>
            {hasSourceCode ? (
              <div className="rounded-xl border border-border p-5">
                <div className="flex items-center gap-2">
                  <Github className="size-4" aria-hidden />
                  <p className="font-medium">私有源码仓库</p>
                </div>
                <p className="mt-2 break-all text-sm leading-6 text-text-muted">
                  {publication
                    ? `${publication.owner}/${publication.repoName}`
                    : '将识别到的源码清理后，按实验室序号创建私有仓库。'}
                </p>
                {publication?.errorMessage && (
                  <p className="mt-3 text-sm leading-6 text-cat-internet">
                    {publication.errorMessage}
                  </p>
                )}
                <div className="mt-4">
                  {publication?.status === 'completed' && publication.repoUrl ? (
                    <Button asChild variant="outline" className="w-full">
                      <a href={publication.repoUrl} target="_blank" rel="noreferrer">
                        <ShieldCheck aria-hidden />
                        打开私有仓库
                      </a>
                    </Button>
                  ) : (
                    <Button
                      className="w-full"
                      disabled={
                        !publishReady ||
                        publishingGitHub ||
                        publication?.status === 'queued' ||
                        publication?.status === 'running'
                      }
                      onClick={onPublishGitHub}
                    >
                      {publishingGitHub ||
                      publication?.status === 'queued' ||
                      publication?.status === 'running' ? (
                        <LoaderCircle className="animate-spin" aria-hidden />
                      ) : (
                        <Github aria-hidden />
                      )}
                      {publication?.status === 'failed'
                        ? '重新尝试发布'
                        : publication
                          ? '正在创建私有仓库'
                          : publishReady
                            ? '创建私有源码仓库'
                            : '等待管理员配置'}
                    </Button>
                  )}
                </div>
              </div>
            ) : null}
            <div className="rounded-xl border border-border p-5">
              <p className="font-medium">任务输入</p>
              <ul className="mt-3 space-y-3">
                {job.inputs.map((input) => (
                  <li key={input.id} className="flex gap-3 text-sm">
                    {input.provider === 'github' ? (
                      <Github className="mt-0.5 size-4 shrink-0" aria-hidden />
                    ) : input.inputKind === 'file' ? (
                      <Archive className="mt-0.5 size-4 shrink-0" aria-hidden />
                    ) : input.inputKind === 'prompt' ? (
                      <FileText className="mt-0.5 size-4 shrink-0" aria-hidden />
                    ) : (
                      <Link2 className="mt-0.5 size-4 shrink-0" aria-hidden />
                    )}
                    <div className="min-w-0">
                      <p className="truncate">{input.displayName}</p>
                      <p className="mt-0.5 text-xs text-text-faint">{input.status}</p>
                    </div>
                  </li>
                ))}
              </ul>
            </div>
            <div className="rounded-xl border border-amber-200 bg-amber-50/60 p-5">
              <p className="font-medium">需要确认</p>
              <ul className="mt-3 space-y-2 text-sm leading-6 text-text-muted">
                {result.warnings.map((warning) => (
                  <li key={warning}>• {warning}</li>
                ))}
              </ul>
            </div>
            {loading && <p className="text-center text-xs text-text-faint">正在同步最新状态…</p>}
          </aside>
        </div>
      )}
    </div>
  )
}

function SectionHeading({
  index,
  title,
  description,
}: {
  index: string
  title: string
  description: string
}) {
  return (
    <div className="flex gap-4">
      <span className="pt-1 font-mono text-xs text-text-faint">{index}</span>
      <div>
        <h2 className="font-serif text-2xl font-semibold">{title}</h2>
        <p className="mt-1 text-sm leading-6 text-text-muted">{description}</p>
      </div>
    </div>
  )
}

function StatusRow({ label, value }: { label: string; value: string }) {
  return (
    <div className="flex justify-between gap-3">
      <dt className="text-text-muted">{label}</dt>
      <dd>{value}</dd>
    </div>
  )
}

function groupArtifacts(artifacts: ImportArtifact[]): Record<string, ImportArtifact[]> {
  return artifacts.reduce<Record<string, ImportArtifact[]>>((groups, artifact) => {
    ;(groups[artifact.artifactKind] ??= []).push(artifact)
    return groups
  }, {})
}

function kindLabel(kind: string): string {
  return (
    (
      {
        code: '源码',
        document: '文档',
        presentation: 'PPT / 演示',
        video: '展示视频',
        image: '图片',
        archive: '压缩文件',
        data: '数据',
        other: '其他',
      } as Record<string, string>
    )[kind] ?? kind
  )
}

function ArtifactIcon({ kind }: { kind: ImportArtifact['artifactKind'] }) {
  const className = 'size-4 text-text-muted'
  if (kind === 'code') return <FileCode2 className={className} aria-hidden />
  if (kind === 'document') return <FileText className={className} aria-hidden />
  if (kind === 'presentation') return <Presentation className={className} aria-hidden />
  if (kind === 'video') return <Film className={className} aria-hidden />
  if (kind === 'image') return <ImageIcon className={className} aria-hidden />
  if (kind === 'archive') return <Archive className={className} aria-hidden />
  return <FileQuestion className={className} aria-hidden />
}

function editorDraft(job: ImportJob): Partial<ProjectWriteInput> {
  const draft = job.result!.projectDraft
  const githubResource: ProjectResourceInput[] =
    job.githubPublication?.status === 'completed' && job.githubPublication.repoUrl
      ? [
          {
            type: 'github',
            title: 'GitHub 私有源码仓库',
            url: job.githubPublication.repoUrl,
          },
        ]
      : []
  return {
    name: draft.name,
    slug: draft.slug,
    summary: draft.summary,
    primaryCategory: draft.primaryCategory,
    highestAward: draft.highestAward,
    status: '研发中',
    sourceName: draft.sourceName ?? null,
    ownerName: draft.ownerName ?? null,
    tags: draft.suggestedTags,
    resources: [
      ...githubResource,
      ...job.artifacts
        .filter((artifact) =>
          ['document', 'presentation', 'video', 'image', 'archive'].includes(
            artifact.artifactKind,
          ),
        )
        .slice(0, 12)
        .map(artifactResource),
    ],
  }
}

function githubPublicationStatus(status: string | undefined, ready: boolean): string {
  if (status === 'queued') return '排队中'
  if (status === 'running') return '正在发布'
  if (status === 'completed') return '已创建'
  if (status === 'failed') return '可重试'
  return ready ? '可创建' : '待凭据'
}

function artifactResource(artifact: ImportArtifact): ProjectResourceInput {
  const type = (
    {
      document: 'document',
      presentation: 'presentation',
      video: 'video',
      image: 'image',
      archive: 'archive',
    } as const
  )[artifact.artifactKind as 'document' | 'presentation' | 'video' | 'image' | 'archive']
  return {
    type,
    title: artifact.relativePath.split('/').at(-1) ?? artifact.relativePath,
    url: null,
  }
}

function formatBytes(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`
  return `${(bytes / 1024 / 1024).toFixed(1)} MB`
}

function parseLinks(value: string): ImportLinkInput[] {
  const found = value.match(/https?:\/\/[^\s<>"']+/gi) ?? []
  const unique = new Set<string>()
  for (const candidate of found) {
    const cleaned = candidate.replace(/[，。；;、,）)\]}]+$/g, '')
    try {
      const parsed = new URL(cleaned)
      if (['http:', 'https:'].includes(parsed.protocol)) unique.add(parsed.toString())
    } catch {
      // 输入过程中允许出现尚未完成的链接。
    }
  }
  return [...unique].map((url) => ({ url }))
}

function shortLink(value: string): string {
  try {
    const url = new URL(value)
    const path = url.pathname === '/' ? '' : url.pathname.replace(/\/$/, '')
    return `${url.hostname}${path}`
  } catch {
    return value
  }
}

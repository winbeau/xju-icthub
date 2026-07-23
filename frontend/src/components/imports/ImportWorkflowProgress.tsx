import {
  AlertTriangle,
  Bot,
  Check,
  Circle,
  FileSearch,
  FolderArchive,
  ListChecks,
  LoaderCircle,
  ShieldCheck,
  Sparkles,
} from 'lucide-react'
import type { ImportJob } from '@/api/schemas/importJob'
import {
  type ImportWorkflowStepState,
  workflowSteps,
} from '@/components/imports/importWorkflowState'

const STEP_DEFINITIONS = [
  { key: 'collect', title: '接收材料', icon: FolderArchive },
  { key: 'extract', title: '解包索引', icon: ShieldCheck },
  { key: 'preview', title: '生成预览', icon: FileSearch },
  { key: 'agent', title: 'Codex 分析', icon: Sparkles },
  { key: 'confirm', title: '人工确认', icon: ListChecks },
] as const

export function ImportWorkflowProgress({
  job,
  showEvents = true,
  submitting = false,
  uploadProgress = 0,
}: {
  job: ImportJob | undefined
  showEvents?: boolean
  submitting?: boolean
  uploadProgress?: number
}) {
  const mappedUploadProgress = Math.min(8, 1 + Math.round((uploadProgress / 100) * 7))
  const progress = submitting ? mappedUploadProgress : (job?.progress ?? 0)
  const summary = workflowSummary(job, submitting, uploadProgress)
  const steps = workflowSteps(job, submitting)
  const recentEvents = job?.events.slice(-4).reverse() ?? []
  const activeRun = job?.agentRuns.at(-1)

  return (
    <div>
      <div className="grid gap-5 rounded-xl bg-bg-subtle p-4 sm:grid-cols-[128px_minmax(0,1fr)] sm:items-center sm:p-5">
        <div className="relative mx-auto flex size-28 items-center justify-center">
          {summary.tone === 'active' && (
            <span className="absolute inset-0 rounded-full border border-dashed border-text/35 motion-safe:animate-spin motion-safe:[animation-duration:7s]" />
          )}
          <svg className="absolute inset-2 size-24 -rotate-90" viewBox="0 0 100 100" aria-hidden>
            <circle
              cx="50"
              cy="50"
              r="43"
              fill="none"
              stroke="currentColor"
              strokeWidth="5"
              className="text-border"
            />
            <circle
              cx="50"
              cy="50"
              r="43"
              fill="none"
              stroke="currentColor"
              strokeLinecap="round"
              strokeWidth="5"
              className={ringTone(summary.tone)}
              pathLength="100"
              strokeDasharray="100"
              strokeDashoffset={100 - progress}
            />
          </svg>
          <div className="relative text-center">
            {summary.tone === 'active' ? (
              <LoaderCircle className="mx-auto size-5 animate-spin" aria-hidden />
            ) : summary.tone === 'success' ? (
              <Check className="mx-auto size-5" aria-hidden />
            ) : summary.tone === 'warning' || summary.tone === 'error' ? (
              <AlertTriangle className="mx-auto size-5" aria-hidden />
            ) : (
              <Bot className="mx-auto size-5" aria-hidden />
            )}
            <p className="mt-1 font-mono text-sm font-semibold">{progress}%</p>
          </div>
        </div>

        <div className="min-w-0">
          <div className="flex flex-wrap items-center gap-2">
            <p className="font-medium">{summary.title}</p>
            {activeRun && (
              <span className="rounded-full border border-border bg-bg px-2.5 py-1 font-mono text-[11px] text-text-muted">
                {activeRun.model}
              </span>
            )}
          </div>
          <p className="mt-2 text-sm leading-6 text-text-muted">{summary.detail}</p>
          {job?.sourceName && (
            <p className="mt-2 truncate text-xs text-text-faint">{job.sourceName}</p>
          )}
        </div>
      </div>

      <div className="mt-5 overflow-x-auto pb-1">
        <ol className="grid min-w-[620px] grid-cols-5">
          {STEP_DEFINITIONS.map((definition, index) => {
            const Icon = definition.icon
            const state = steps[index] ?? 'pending'
            return (
              <li
                key={definition.key}
                className="relative flex flex-col items-center px-2 text-center"
              >
                {index > 0 && (
                  <span
                    className={`absolute right-1/2 top-4 h-px w-full ${state === 'pending' ? 'bg-border' : 'bg-text/35'}`}
                    aria-hidden
                  />
                )}
                <span
                  className={`relative z-10 flex size-9 items-center justify-center rounded-full border ${stepTone(state)}`}
                >
                  {state === 'complete' ? (
                    <Check className="size-4" aria-hidden />
                  ) : state === 'active' ? (
                    <span className="relative flex size-full items-center justify-center">
                      <span className="absolute inset-[-5px] rounded-full border border-text/25 motion-safe:animate-ping" />
                      <Icon className="size-4" aria-hidden />
                    </span>
                  ) : state === 'warning' || state === 'failed' ? (
                    <AlertTriangle className="size-4" aria-hidden />
                  ) : (
                    <Circle className="size-3" aria-hidden />
                  )}
                </span>
                <p
                  className={`mt-2 text-xs font-medium ${state === 'pending' ? 'text-text-faint' : 'text-text'}`}
                >
                  {definition.title}
                </p>
                <p className="mt-1 text-[11px] text-text-faint">{stepLabel(state)}</p>
              </li>
            )
          })}
        </ol>
      </div>

      {showEvents && recentEvents.length > 0 && (
        <div className="mt-5 border-t border-border pt-4">
          <div className="flex items-center justify-between gap-3">
            <p className="text-xs font-medium uppercase tracking-[0.12em] text-text-faint">
              实时记录
            </p>
            <p className="text-xs text-text-faint">最近 {recentEvents.length} 条</p>
          </div>
          <ol className="mt-3 space-y-2">
            {recentEvents.map((event, index) => (
              <li key={event.id} className="flex items-start gap-3 text-sm">
                <span
                  className={`mt-2 size-1.5 shrink-0 rounded-full ${index === 0 && summary.tone === 'active' ? 'bg-text motion-safe:animate-pulse' : 'bg-border-strong'}`}
                />
                <div className="min-w-0 flex-1">
                  <div className="flex items-baseline justify-between gap-3">
                    <p className="truncate text-text-muted">{event.stage}</p>
                    <time className="shrink-0 font-mono text-[11px] text-text-faint">
                      {formatEventTime(event.createdAt)}
                    </time>
                  </div>
                  {event.message && (
                    <p className="mt-0.5 text-xs leading-5 text-text-faint">{event.message}</p>
                  )}
                </div>
              </li>
            ))}
          </ol>
        </div>
      )}
    </div>
  )
}

function workflowSummary(
  job: ImportJob | undefined,
  submitting: boolean,
  uploadProgress: number,
): { title: string; detail: string; tone: 'idle' | 'active' | 'success' | 'warning' | 'error' } {
  if (!job) {
    return submitting
      ? {
          title: '正在接收材料',
          detail: '附件和链接正在上传，请保持当前页面打开。',
          tone: 'active',
        }
      : { title: '等待开始', detail: '提交后会在这里显示每一步的实时状态。', tone: 'idle' }
  }
  if (job.status === 'uploading')
    return {
      title: '正在分片接收材料',
      detail: `附件已上传 ${uploadProgress}%，网络波动时会自动重试当前小块。`,
      tone: 'active',
    }
  const latest = job.events.at(-1)
  if (job.status === 'failed')
    return {
      title: job.stage,
      detail: job.errorMessage ?? latest?.message ?? '整理未完成，请检查材料后重试。',
      tone: 'error',
    }
  if (job.status === 'cancelled')
    return {
      title: '整理已取消',
      detail: latest?.message ?? '已填写内容仍保留在当前草稿中。',
      tone: 'warning',
    }
  if (job.events.some((event) => event.eventType === 'agent_fallback'))
    return {
      title: '已保留本地草稿',
      detail: latest?.message ?? 'Codex 本次未完成，可以继续人工确认或稍后重试。',
      tone: 'warning',
    }
  if (job.events.some((event) => event.eventType === 'agent_completed'))
    return {
      title: 'Codex 草稿已生成',
      detail: latest?.message ?? '结构化结果已就绪，等待人工确认。',
      tone: 'success',
    }
  if (job.status === 'completed')
    return {
      title: '本地预览已就绪',
      detail: '可补充要求并交给 Codex 进一步理解项目材料。',
      tone: 'idle',
    }
  return { title: job.stage, detail: latest?.message ?? '后台正在处理项目材料。', tone: 'active' }
}

function ringTone(tone: ReturnType<typeof workflowSummary>['tone']): string {
  if (tone === 'warning') return 'text-amber-600'
  if (tone === 'error') return 'text-red-600'
  if (tone === 'success') return 'text-emerald-700'
  return 'text-text'
}

function stepTone(state: ImportWorkflowStepState): string {
  if (state === 'complete') return 'border-text bg-text text-bg'
  if (state === 'active') return 'border-text bg-bg text-text'
  if (state === 'warning') return 'border-amber-300 bg-amber-50 text-amber-700'
  if (state === 'failed') return 'border-red-300 bg-red-50 text-red-700'
  return 'border-border bg-bg text-text-faint'
}

function stepLabel(state: ImportWorkflowStepState): string {
  if (state === 'complete') return '完成'
  if (state === 'active') return '进行中'
  if (state === 'warning') return '已回退'
  if (state === 'failed') return '失败'
  return '等待'
}

function formatEventTime(value: string): string {
  const normalized = value.includes('T') ? value : `${value.replace(' ', 'T')}Z`
  const date = new Date(normalized)
  if (Number.isNaN(date.getTime())) return value.slice(11, 16)
  return new Intl.DateTimeFormat('zh-CN', {
    hour: '2-digit',
    minute: '2-digit',
    second: '2-digit',
    hour12: false,
  }).format(date)
}

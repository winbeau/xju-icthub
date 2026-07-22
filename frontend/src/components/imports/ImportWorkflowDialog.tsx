import { useEffect } from 'react'
import { createPortal } from 'react-dom'
import {
  Bot,
  Check,
  FileSearch,
  FolderArchive,
  ListChecks,
  LoaderCircle,
  Save,
  ShieldCheck,
  Sparkles,
  Square,
  X,
} from 'lucide-react'
import type { ImportJob } from '@/api/schemas/importJob'
import { Button } from '@/components/ui/button'
import { Textarea } from '@/components/ui/textarea'

const STEPS = [
  {
    title: '收集输入',
    detail: '接收项目简介、多个链接和混合附件。',
    icon: FolderArchive,
  },
  {
    title: '安全整理',
    detail: '隔离文件，检查路径与体积，展开 ZIP 并建立索引。',
    icon: ShieldCheck,
  },
  {
    title: '理解材料',
    detail: '读取源码结构、文档、PPT 和媒体信息；不执行项目代码。',
    icon: FileSearch,
  },
  {
    title: '生成草稿',
    detail: '整理项目名、简介、主分类和资源链接，保留依据不足的字段为空。',
    icon: Sparkles,
  },
  {
    title: '人工确认',
    detail: '成员核对并补充内容，确认后才正式上传项目。',
    icon: ListChecks,
  },
] as const

export function ImportWorkflowDialog({
  additionalPrompt,
  cancelling,
  fileCount,
  job,
  linkCount,
  onAdditionalPromptChange,
  onCancel,
  onOpenChange,
  onSavePrompt,
  open,
  promptProvided,
  savingPrompt,
  submitting,
}: {
  additionalPrompt: string
  cancelling: boolean
  fileCount: number
  job: ImportJob | undefined
  linkCount: number
  onAdditionalPromptChange: (value: string) => void
  onCancel: (() => void) | undefined
  onOpenChange: (open: boolean) => void
  onSavePrompt: (() => void) | undefined
  open: boolean
  promptProvided: boolean
  savingPrompt: boolean
  submitting: boolean
}) {
  useEffect(() => {
    if (!open) return
    const previousOverflow = document.body.style.overflow
    document.body.style.overflow = 'hidden'
    const closeOnEscape = (event: KeyboardEvent) => {
      if (event.key === 'Escape') onOpenChange(false)
    }
    window.addEventListener('keydown', closeOnEscape)
    return () => {
      document.body.style.overflow = previousOverflow
      window.removeEventListener('keydown', closeOnEscape)
    }
  }, [onOpenChange, open])

  if (!open) return null
  const currentStep = workflowStep(job, submitting)
  const failed = job?.status === 'failed'
  const completed = job?.status === 'completed'

  return createPortal(
    <div
      className="fixed inset-0 z-50 flex items-center justify-center bg-black/30 px-4 py-8 backdrop-blur-[2px]"
      onMouseDown={(event) => {
        if (event.target === event.currentTarget) onOpenChange(false)
      }}
    >
      <section
        role="dialog"
        aria-modal="true"
        aria-labelledby="import-workflow-title"
        className="max-h-full w-full max-w-4xl overflow-y-auto rounded-2xl border border-border bg-bg shadow-2xl"
      >
        <header className="flex items-start gap-4 border-b border-border px-5 py-5 sm:px-6">
          <div className="rounded-lg bg-bg-subtle p-2.5">
            <Bot className="size-5" aria-hidden />
          </div>
          <div className="min-w-0 flex-1">
            <p className="text-xs font-medium uppercase tracking-[0.14em] text-text-faint">
              Import Workflow
            </p>
            <h2 id="import-workflow-title" className="mt-1 font-serif text-2xl font-semibold">
              Codex 如何整理项目
            </h2>
            <p className="mt-2 text-sm text-text-muted">
              {job?.stage ?? (submitting ? '正在提交材料' : '等待开始')}
            </p>
          </div>
          <Button
            type="button"
            variant="ghost"
            size="icon"
            aria-label="关闭工作流程"
            onClick={() => onOpenChange(false)}
          >
            <X aria-hidden />
          </Button>
        </header>

        <div className="px-5 py-5 sm:px-6">
          <div className="grid grid-cols-3 divide-x divide-border rounded-lg bg-bg-subtle py-3 text-center">
            <InputCount label="简介" value={promptProvided ? '已填写' : '未填写'} />
            <InputCount label="链接" value={`${linkCount} 条`} />
            <InputCount label="附件" value={`${fileCount} 个`} />
          </div>

          <div className="mt-5">
            <label htmlFor="codex-refinement" className="text-sm font-medium">
              补充给 Codex 的提示
            </label>
            <p className="mt-1 text-sm leading-6 text-text-muted">
              可补充希望识别的标签、负责人、来源、奖项或其他整理要求。流程完成后再注入 Codex，不会中断当前文件整理。
            </p>
            <Textarea
              id="codex-refinement"
              value={additionalPrompt}
              onChange={(event) => onAdditionalPromptChange(event.target.value)}
              placeholder={'例如：\n标签：Web、校园服务\n负责人：张三\n来源：课程项目'}
              className="mt-3 min-h-24 text-sm leading-6"
            />
          </div>

          <ol className="mt-5 grid gap-3 sm:grid-cols-5">
            {STEPS.map((step, index) => {
              const Icon = step.icon
              const completed = index < currentStep || (job?.status === 'completed' && index <= 3)
              const active = index === currentStep && !failed
              const failedStep = failed && index === currentStep
              return (
                <li
                  key={step.title}
                  className={`relative rounded-lg border p-3 ${active ? 'border-border-strong bg-bg-subtle' : 'border-border'}`}
                >
                  <span
                    className={`flex size-8 shrink-0 items-center justify-center rounded-full border ${completed ? 'border-text bg-text text-bg' : failedStep ? 'border-red-300 bg-red-50 text-red-700' : active ? 'border-text bg-bg text-text' : 'border-border bg-bg text-text-faint'}`}
                  >
                    {completed ? <Check className="size-4" aria-hidden /> : <Icon className="size-4" aria-hidden />}
                  </span>
                  <div className="mt-3">
                    <p className={`text-sm font-medium ${active ? 'text-text' : 'text-text-muted'}`}>
                      {index + 1}. {step.title}
                    </p>
                    <p className="mt-1.5 text-xs leading-5 text-text-muted">{step.detail}</p>
                    {active && <span className="mt-2 inline-block text-xs text-text-faint">进行中</span>}
                    {failedStep && <span className="mt-2 inline-block text-xs text-red-700">已停止</span>}
                  </div>
                </li>
              )
            })}
          </ol>

          <div className="mt-5 flex flex-col gap-3 border-t border-border pt-4 sm:flex-row sm:items-center sm:justify-between">
            <p className="text-xs leading-5 text-text-faint">
              取消任务只停止本次整理；已经填写的简介、链接、附件和补充提示会保留在当前草稿中。
            </p>
            <div className="flex shrink-0 justify-end gap-2">
              {onCancel && (
                <Button
                  type="button"
                  variant="destructive"
                  disabled={cancelling}
                  onClick={onCancel}
                >
                  {cancelling ? <LoaderCircle className="animate-spin" aria-hidden /> : <Square aria-hidden />}
                  {cancelling ? '正在取消…' : '取消整理'}
                </Button>
              )}
              <Button
                type="button"
                disabled={!completed || !additionalPrompt.trim() || savingPrompt}
                onClick={onSavePrompt}
              >
                {savingPrompt ? <LoaderCircle className="animate-spin" aria-hidden /> : completed ? <Save aria-hidden /> : <Sparkles aria-hidden />}
                {savingPrompt ? '正在保存…' : completed ? '保存并交给 Codex' : '等待整理完成'}
              </Button>
            </div>
          </div>
        </div>
      </section>
    </div>,
    document.body,
  )
}

function InputCount({ label, value }: { label: string; value: string }) {
  return (
    <div className="px-2">
      <p className="text-xs text-text-faint">{label}</p>
      <p className="mt-1 text-sm font-medium">{value}</p>
    </div>
  )
}

function workflowStep(job: ImportJob | undefined, submitting: boolean): number {
  if (submitting && !job) return 1
  if (!job) return 0
  if (job.status === 'failed') return job.progress >= 70 ? 3 : job.progress >= 15 ? 1 : 0
  if (job.status === 'completed') return 4
  if (job.status === 'analyzing') return 2
  if (job.status === 'extracting') return 1
  return job.progress >= 70 ? 3 : job.progress >= 15 ? 1 : 0
}

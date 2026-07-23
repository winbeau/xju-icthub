import type { ImportJob } from '@/api/schemas/importJob'

export type ImportWorkflowStepState = 'complete' | 'active' | 'pending' | 'warning' | 'failed'

export function workflowSteps(
  job: ImportJob | undefined,
  submitting = false,
): ImportWorkflowStepState[] {
  if (!job)
    return submitting
      ? ['active', 'pending', 'pending', 'pending', 'pending']
      : ['pending', 'pending', 'pending', 'pending', 'pending']
  if (job.status === 'uploading') return ['active', 'pending', 'pending', 'pending', 'pending']
  const types = new Set(job.events.map((event) => event.eventType))
  const agentStarted =
    types.has('agent_queued') || job.status === 'agent_queued' || job.status === 'agent_running'
  const agentCompleted = types.has('agent_completed')
  const agentFallback = types.has('agent_fallback')
  const terminalFailure = job.status === 'failed'

  const collect: ImportWorkflowStepState = 'complete'
  const extract: ImportWorkflowStepState =
    terminalFailure && job.progress < 70
      ? 'failed'
      : job.progress >= 70 || types.has('completed')
        ? 'complete'
        : 'active'
  const preview: ImportWorkflowStepState =
    terminalFailure && job.progress >= 70
      ? 'failed'
      : job.result || agentStarted
        ? 'complete'
        : job.progress >= 70
          ? 'active'
          : 'pending'
  const agent: ImportWorkflowStepState = agentFallback
    ? 'warning'
    : agentCompleted
      ? 'complete'
      : job.status === 'agent_running' || job.status === 'agent_queued'
        ? 'active'
        : 'pending'
  const confirm: ImportWorkflowStepState = agentCompleted || agentFallback ? 'active' : 'pending'

  return [collect, extract, preview, agent, confirm]
}

export function agentStatusLabel(job: ImportJob): string {
  const run = job.agentRuns.at(-1)
  if (run?.status === 'running' || job.status === 'agent_running') return '运行中'
  if (job.status === 'agent_queued') return '已排队'
  if (
    run?.status === 'completed' ||
    job.events.some((event) => event.eventType === 'agent_completed')
  )
    return '已完成'
  if (run?.status === 'failed' || job.events.some((event) => event.eventType === 'agent_fallback'))
    return '已回退'
  return '等待补充提示'
}

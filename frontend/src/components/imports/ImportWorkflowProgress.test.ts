import { describe, expect, it } from 'vitest'
import type { ImportJob } from '@/api/schemas/importJob'
import { agentStatusLabel, workflowSteps } from '@/components/imports/importWorkflowState'

function job(overrides: Partial<ImportJob> = {}): ImportJob {
  return {
    id: 'job-1',
    status: 'completed',
    stage: '等待确认',
    progress: 100,
    sourceKind: 'zip',
    sourceName: 'project.zip',
    analysisEngine: 'deterministic_fallback',
    errorMessage: null,
    createdAt: '2026-07-23 05:58:38',
    updatedAt: '2026-07-23 05:58:39',
    attemptCount: 1,
    startedAt: '2026-07-23 05:58:39',
    completedAt: '2026-07-23 05:58:39',
    analysisBundlePath: null,
    agentThreadId: null,
    inputs: [],
    artifacts: [],
    events: [
      {
        id: 1,
        eventType: 'queued',
        status: 'queued',
        stage: '等待解析',
        progress: 5,
        message: null,
        createdAt: '2026-07-23 05:58:38',
      },
      {
        id: 2,
        eventType: 'completed',
        status: 'completed',
        stage: '等待确认',
        progress: 100,
        message: null,
        createdAt: '2026-07-23 05:58:39',
      },
    ],
    agentRuns: [],
    result: {
      projectDraft: {
        name: '测试项目',
        slug: 'test-project',
        summary: '简介',
        primaryCategory: '传统软件',
        suggestedTags: [],
        highestAward: null,
        status: '正在研发',
      },
      artifactSummary: [],
      normalizedResources: {
        sourceCode: [],
        documents: [],
        presentations: [],
        videos: [],
        links: [],
      },
      warnings: [],
      agent: { status: 'awaiting_prompt', mode: 'deterministic_fallback', message: '等待补充提示' },
      capabilities: {
        zipUpload: 'ready',
        githubLink: 'planned',
        mixedFiles: 'planned',
        codexAgent: 'ready',
        githubPublish: 'planned',
      },
    },
    ...overrides,
  }
}

describe('import workflow visualization', () => {
  it('keeps Codex and confirmation pending before refinement', () => {
    expect(workflowSteps(job())).toEqual(['complete', 'complete', 'complete', 'pending', 'pending'])
    expect(agentStatusLabel(job())).toBe('等待补充提示')
  })

  it('shows the Codex node as active while the agent runs', () => {
    const running = job({
      status: 'agent_running',
      progress: 88,
      events: [
        ...job().events,
        {
          id: 3,
          eventType: 'agent_queued',
          status: 'agent_queued',
          stage: '等待 Codex 分析',
          progress: 82,
          message: null,
          createdAt: '2026-07-23 05:59:50',
        },
      ],
      agentRuns: [
        {
          id: 'run-1',
          runner: 'codex_exec',
          model: 'gpt-5.6-sol',
          baseUrlOrigin: 'https://api.selab.top',
          status: 'running',
          rawEventsPath: null,
          errorMessage: null,
          startedAt: '2026-07-23 05:59:51',
          completedAt: null,
          createdAt: '2026-07-23 05:59:51',
        },
      ],
    })
    expect(workflowSteps(running)).toEqual([
      'complete',
      'complete',
      'complete',
      'active',
      'pending',
    ])
    expect(agentStatusLabel(running)).toBe('运行中')
  })

  it('renders fallback as a warning while keeping confirmation available', () => {
    const fallback = job({
      events: [
        ...job().events,
        {
          id: 3,
          eventType: 'agent_fallback',
          status: 'completed',
          stage: '使用本地草稿',
          progress: 100,
          message: 'Codex 未完成',
          createdAt: '2026-07-23 06:00:00',
        },
      ],
      agentRuns: [
        {
          id: 'run-1',
          runner: 'codex_exec',
          model: 'gpt-5.6-sol',
          baseUrlOrigin: 'https://api.selab.top',
          status: 'failed',
          rawEventsPath: null,
          errorMessage: 'Codex 未完成',
          startedAt: '2026-07-23 05:59:51',
          completedAt: '2026-07-23 06:00:00',
          createdAt: '2026-07-23 05:59:51',
        },
      ],
    })
    expect(workflowSteps(fallback)).toEqual([
      'complete',
      'complete',
      'complete',
      'warning',
      'active',
    ])
    expect(agentStatusLabel(fallback)).toBe('已回退')
  })
})

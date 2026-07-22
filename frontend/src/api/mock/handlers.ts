import { ApiError, registerMock } from '@/api/client'
import { PROJECT_FIXTURES } from '@/api/mock/fixtures'
import { LoginRequestSchema, type User } from '@/api/schemas/user'
import {
  ProjectCategorySchema,
  ProjectWriteInputSchema,
  type ProjectDetail,
  type ProjectWriteInput,
} from '@/api/schemas/project'
import { TagCreateInputSchema, type TagDefinition } from '@/api/schemas/tag'
import { buildCoverPreview } from '@/lib/covers'
import { canManageTags } from '@/api/schemas/user'
import { filterProjects } from '@/lib/projects'
import type { ImportJob } from '@/api/schemas/importJob'

function mockUser(sid: string): User {
  const superadmin = sid === '20211019999'
  const member = superadmin || sid !== '20211010000'
  return {
    sid,
    name: superadmin ? '超级管理员' : member ? '实验室成员' : '飞跃用户',
    nickname: superadmin ? '超级管理员' : member ? 'ICT 成员' : '飞跃用户',
    preferredName: null,
    avatar: null,
    avatarThumb: null,
    bio: null,
    wechat: null,
    phone: null,
    email: null,
    role: superadmin ? 'superadmin' : 'user',
    isAdmin: superadmin,
    isSuperAdmin: superadmin,
    isLabMember: member,
    classId: null,
    classFullName: null,
    classShortName: null,
    isClassCommittee: false,
    committeeTitle: null,
  }
}

let mockProjects: ProjectDetail[] = PROJECT_FIXTURES.map((project) => ({
  ...project,
  tags: [...project.tags],
  resources: project.resources.map((resource) => ({ ...resource })),
}))

let mockTags: TagDefinition[] = [
  ['competition-innovation', '国创赛（互联网+）', '比赛'],
  ['competition-design', '计算机设计大赛', '比赛'],
  ['competition-intelligent', '智能应用技术大赛', '比赛'],
  ['tech-big-data', '大数据', '技术'], ['tech-ai', '人工智能应用', '技术'],
  ['tech-agent', 'LLM/Agent', '技术'], ['tech-cv', '计算机视觉', '技术'],
  ['tech-nlp', 'NLP', '技术'], ['tech-iot', '物联网', '技术'],
  ['tech-embedded', '嵌入式', '技术'], ['tech-robot', '机器人', '技术'],
  ['tech-web', 'Web', '技术'], ['tech-mobile', '移动端', '技术'], ['tech-3d', '3D/VR', '技术'],
  ['feature-hybrid', '软硬结合', '特征'], ['feature-ai-core', 'AI核心', '特征'],
  ['feature-ai-enhanced', 'AI增强', '特征'], ['feature-non-ai', '非AI', '特征'], ['feature-open', '开源项目', '特征'],
  ['domain-campus', '校园服务', '领域'], ['domain-education', '教育', '领域'], ['domain-agriculture', '农业', '领域'],
  ['domain-medical', '医疗', '领域'], ['domain-tourism', '文旅', '领域'], ['domain-industry', '工业', '领域'], ['domain-research', '科研辅助', '领域'],
  ['source-competition', '比赛项目', '来源'], ['source-lab', '实验室建设', '来源'], ['source-course', '课程项目', '来源'],
  ['source-tool', '日常工具', '来源'], ['source-personal', '个人探索', '来源'], ['source-service', '对外服务', '来源'],
].map(([id, name, groupName], index) => ({ id: id!, name: name!, groupName: groupName!, color: null, sortOrder: index, isActive: true, mergedIntoId: null }))

const mockImportJobs: Record<string, ImportJob> = {}

function requireMember(headers: Headers): User {
  const token = headers.get('Authorization')?.replace(/^Bearer\s+/i, '')
  const sid = token?.startsWith('mock:') ? token.slice(5) : null
  if (!sid) throw new ApiError('请先登录', 401, '/api/v1/projects')
  const user = mockUser(sid)
  if (!user.isLabMember && !user.isSuperAdmin) {
    throw new ApiError('当前账号尚未开通实验室权限，请联系管理员', 403, '/api/v1/projects')
  }
  return user
}

function detailFromInput(
  input: ProjectWriteInput,
  id: string = crypto.randomUUID(),
): ProjectDetail {
  const generated = buildCoverPreview(input)
  return {
    id,
    ...input,
    classificationStatus: 'classified',
    coverMode: input.coverMode,
    coverResourceId: null,
    coverResourceUrl: generated.coverResourceUrl,
    coverTitle: input.coverTitle ?? generated.coverTitle,
    coverSubtitle: input.coverSubtitle ?? generated.coverSubtitle,
    coverKeywords: input.coverKeywords.length ? input.coverKeywords : generated.coverKeywords,
    coverTone: input.coverTone,
    coverConfidence: input.coverMode === 'manual' ? 1 : generated.coverConfidence,
    resources: input.resources.map((resource) => ({ id: crypto.randomUUID(), ...resource })),
  }
}

registerMock('POST', '/auth/login', ({ body }) => {
  const credentials = LoginRequestSchema.parse(body)
  if (credentials.password !== '123456') {
    throw new ApiError('学号或密码不正确', 401, '/auth/login')
  }
  return {
    user: mockUser(credentials.sid),
    token: `mock:${credentials.sid}`,
  }
})

registerMock('POST', '/auth/logout', () => null)

registerMock('GET', '/auth/me', ({ headers }) => {
  const token = headers.get('Authorization')?.replace(/^Bearer\s+/i, '')
  const sid = token?.startsWith('mock:') ? token.slice(5) : null
  if (!sid) throw new ApiError('登录已过期', 401, '/auth/me')
  return mockUser(sid)
})

registerMock('GET', '/api/v1/projects', ({ query, headers }) => {
  requireMember(headers)
  const categoryRaw = query.get('category')
  const category = categoryRaw ? ProjectCategorySchema.parse(categoryRaw) : undefined
  const items = filterProjects(mockProjects, query.get('q') ?? undefined, category)
  return { items, total: items.length }
})

registerMock('GET', '/api/v1/projects/:slug', ({ path, headers }) => {
  requireMember(headers)
  const slug = decodeURIComponent(path.split('/').at(-1) ?? '')
  const project = mockProjects.find((item) => item.slug === slug)
  if (!project) throw new ApiError('项目不存在', 404, path)
  return project
})

registerMock('POST', '/api/v1/projects', ({ body, headers }) => {
  requireMember(headers)
  const input = ProjectWriteInputSchema.parse(body)
  if (mockProjects.some((project) => project.slug === input.slug)) {
    throw new ApiError(`项目路径 ${input.slug} 已存在`, 409, '/api/v1/projects')
  }
  const project = detailFromInput(input)
  mockProjects = [project, ...mockProjects]
  return project
})

registerMock('PUT', '/api/v1/projects/:slug', ({ body, headers, path }) => {
  requireMember(headers)
  const currentSlug = decodeURIComponent(path.split('/').at(-1) ?? '')
  const index = mockProjects.findIndex((project) => project.slug === currentSlug)
  if (index < 0) throw new ApiError('项目不存在', 404, path)
  const input = ProjectWriteInputSchema.parse(body)
  if (
    mockProjects.some((project, itemIndex) => project.slug === input.slug && itemIndex !== index)
  ) {
    throw new ApiError(`项目路径 ${input.slug} 已存在`, 409, path)
  }
  const project = detailFromInput(input, mockProjects[index]!.id)
  mockProjects[index] = project
  return project
})

registerMock('DELETE', '/api/v1/projects/:slug', ({ headers, path }) => {
  requireMember(headers)
  const slug = decodeURIComponent(path.split('/').at(-1) ?? '')
  const index = mockProjects.findIndex((project) => project.slug === slug)
  if (index < 0) throw new ApiError('项目不存在', 404, path)
  mockProjects.splice(index, 1)
  return null
})

registerMock('POST', '/api/v1/projects/import', ({ body, headers }) => {
  requireMember(headers)
  const payload = ProjectWriteInputSchema.array()
    .max(200)
    .parse((body as { items?: unknown } | null)?.items)
  let created = 0
  let updated = 0
  for (const input of payload) {
    const index = mockProjects.findIndex((project) => project.slug === input.slug)
    if (index >= 0) {
      mockProjects[index] = detailFromInput(input, mockProjects[index]!.id)
      updated += 1
    } else {
      mockProjects.unshift(detailFromInput(input))
      created += 1
    }
  }
  return { created, updated, total: payload.length }
})

registerMock('GET', '/api/v1/tags', ({ headers }) => {
  requireMember(headers)
  return mockTags.filter((tag) => tag.isActive)
})

registerMock('POST', '/api/v1/tags', ({ body, headers }) => {
  const user = requireMember(headers)
  if (!canManageTags(user)) throw new ApiError('禁止访问', 403, '/api/v1/tags')
  const input = TagCreateInputSchema.parse(body)
  const created: TagDefinition = { id: crypto.randomUUID(), ...input, isActive: true, mergedIntoId: null }
  mockTags = [...mockTags, created]
  return created
})

registerMock('PATCH', '/api/v1/tags/:id', ({ body, headers, path }) => {
  const user = requireMember(headers)
  if (!canManageTags(user)) throw new ApiError('禁止访问', 403, path)
  const id = decodeURIComponent(path.split('/').at(-1) ?? '')
  const tag = mockTags.find((item) => item.id === id)
  if (!tag) throw new ApiError('标签不存在', 404, path)
  Object.assign(tag, body)
  return tag
})

registerMock('POST', '/api/v1/tags/:id/merge', ({ body, headers, path }) => {
  const user = requireMember(headers)
  if (!canManageTags(user)) throw new ApiError('禁止访问', 403, path)
  const id = decodeURIComponent(path.split('/').at(-2) ?? '')
  const targetId = (body as { targetId?: unknown })?.targetId
  const source = mockTags.find((item) => item.id === id)
  const target = mockTags.find((item) => item.id === targetId)
  if (!source || !target) throw new ApiError('标签不存在', 404, path)
  mockProjects.forEach((project) => {
    project.tags = [...new Set(project.tags.map((tag) => tag === source.name ? target.name : tag))]
  })
  source.isActive = false
  source.mergedIntoId = target.id
  return null
})

registerMock('POST', '/api/v1/tag-suggestions', ({ body, headers }) => {
  requireMember(headers)
  const name = (body as { name?: unknown })?.name
  if (typeof name !== 'string' || !name.trim()) throw new ApiError('建议标签名称不能为空', 400, '/api/v1/tag-suggestions')
  return { id: crypto.randomUUID(), status: 'pending' }
})

registerMock('POST', '/api/v1/import-jobs', ({ body, headers }) => {
  requireMember(headers)
  const form = body as FormData
  const files = form.getAll('file').filter((value): value is File => value instanceof File)
  const id = crypto.randomUUID()
  const now = new Date().toISOString()
  const rawPrompt = form.get('prompt')
  const prompt = typeof rawPrompt === 'string' ? rawPrompt.trim() : ''
  const rawLinks = form.get('links')
  const links = typeof rawLinks === 'string'
    ? (JSON.parse(rawLinks) as Array<{ url: string; title?: string }>)
    : []
  if (!files.length && !links.length && !prompt) {
    throw new ApiError('请至少填写简介、链接或上传一个附件', 400, '/api/v1/import-jobs')
  }
  const artifacts = files.map((file) => {
    const kind = mockArtifactKind(file.name)
    return {
      id: crypto.randomUUID(),
      relativePath: file.name,
      artifactKind: kind,
      mimeType: file.type || null,
      sizeBytes: file.size,
      extractor: kind === 'document' ? 'text_preview' : 'file_index',
      isCoverCandidate: kind === 'image',
    }
  })
  const sourceName = files.length
    ? files.length === 1
      ? files[0]!.name
      : `${files[0]!.name} 等 ${files.length} 个附件`
    : links[0]?.url ?? '项目简介'
  const projectName =
    files[0]?.name.replace(/\.[^.]+$/i, '')
    || prompt.match(/(?:项目名|项目名称|名称)\s*[:：]\s*([^\n。；;]+)/)?.[1]?.trim()
    || '待识别项目'
  const job: ImportJob = {
    id,
    status: 'completed',
    stage: '等待确认',
    progress: 100,
    sourceKind:
      files.length === 1 && files[0]!.name.toLowerCase().endsWith('.zip')
        ? 'zip'
        : files.length
          ? 'mixed'
          : links.length
            ? 'link'
            : 'prompt',
    sourceName,
    analysisEngine: 'deterministic_fallback',
    errorMessage: null,
    createdAt: now,
    updatedAt: now,
    inputs: [
      ...files.map((file) => ({
        id: crypto.randomUUID(), inputKind: 'file' as const, provider: 'upload', displayName: file.name,
        sourceRef: null, mimeType: file.type || 'application/octet-stream', sizeBytes: file.size, status: 'parsed',
      })),
      ...(prompt ? [{
        id: crypto.randomUUID(), inputKind: 'prompt' as const, provider: 'user', displayName: '项目简介',
        sourceRef: prompt, mimeType: null, sizeBytes: null, status: 'parsed',
      }] : []),
      ...links.map((link) => ({
        id: crypto.randomUUID(), inputKind: 'link' as const,
        provider: link.url.includes('github.com') ? 'github' : link.url.includes('baidu') ? 'baidu' : 'web',
        displayName: link.title || link.url, sourceRef: link.url, mimeType: null, sizeBytes: null,
        status: 'pending_parser',
      })),
    ],
    artifacts,
    result: {
      projectDraft: {
        name: projectName, slug: `import-${id.slice(0, 8)}`,
        summary: files.length
          ? `项目材料中识别到 ${files.length} 个附件，等待进一步理解与确认。`
          : '已收集项目简介与链接，等待进一步理解与确认。',
        primaryCategory: '传统软件',
        suggestedTags: mockExplicitTags(prompt),
        ownerName: mockExplicitField(prompt, ['当前负责', '负责人', '维护者']),
        sourceName: mockExplicitField(prompt, ['来源者', '来源方', '来源']),
        highestAward: mockExplicitField(prompt, ['最高奖项', '获奖', '奖项']),
        status: '待确认',
      },
      artifactSummary: summarizeMockArtifacts(artifacts),
      warnings: ['当前使用确定性回退生成草稿；配置 Codex 后将补充语义摘要、奖项识别和更准确的分类。', 'PPT、文档和视频已完成文件级归类，内容抽取器将在 Agent 链路配置阶段接入。'],
      agent: { status: 'awaiting_configuration', mode: 'deterministic_fallback', message: '材料收集与安全归类链路已打通，等待配置 Codex Base URL 与 API Token。' },
      capabilities: { zipUpload: 'prototype_ready', githubLink: 'input_reserved', mixedFiles: 'prototype_ready', codexAgent: 'awaiting_configuration', githubPublish: 'awaiting_credentials' },
    },
  }
  mockImportJobs[id] = job
  return job
})

function mockArtifactKind(name: string): ImportJob['artifacts'][number]['artifactKind'] {
  const extension = name.split('.').pop()?.toLowerCase() ?? ''
  if (['ppt', 'pptx', 'key', 'odp'].includes(extension)) return 'presentation'
  if (['pdf', 'doc', 'docx', 'odt', 'rtf', 'md', 'txt', 'tex'].includes(extension)) return 'document'
  if (['mp4', 'mov', 'mkv', 'avi', 'webm', 'm4v', 'wmv'].includes(extension)) return 'video'
  if (['png', 'jpg', 'jpeg', 'gif', 'webp', 'svg', 'bmp', 'tiff'].includes(extension)) return 'image'
  if (['zip', 'rar', '7z', 'tar', 'gz', 'bz2', 'xz'].includes(extension)) return 'archive'
  if (['rs', 'ts', 'tsx', 'js', 'jsx', 'py', 'java', 'go', 'c', 'cpp', 'h', 'hpp', 'html', 'css', 'sql'].includes(extension)) return 'code'
  return 'other'
}

function mockExplicitTags(prompt: string): string[] {
  const formalTags = new Set([
    '国创赛（互联网+）', '计算机设计大赛', '智能应用技术大赛', '大数据',
    '人工智能应用', 'LLM/Agent', '计算机视觉', 'NLP', '物联网', '嵌入式',
    '机器人', 'Web', '移动端', '3D/VR', '软硬结合', 'AI核心', 'AI增强',
    '非AI', '开源项目', '校园服务', '教育', '农业', '医疗', '文旅', '工业',
    '科研辅助', '比赛项目', '实验室建设', '课程项目', '日常工具', '个人探索',
    '对外服务',
  ])
  const value = mockExplicitField(prompt, ['标签', '项目标签'])
  if (!value) return []
  return value
    .split(/[,，、/|]/)
    .map((tag) => tag.trim())
    .filter((tag) => formalTags.has(tag))
    .slice(0, 3)
}

function mockExplicitField(prompt: string, labels: string[]): string | null {
  for (const line of prompt.split(/\r?\n/)) {
    const match = line.trim().match(/^([^:：]+)[:：]\s*(.+)$/)
    if (!match || !labels.includes(match[1]!.trim())) continue
    const value = match[2]!.split(/[。；;]/)[0]?.trim()
    if (value) return [...value].slice(0, 120).join('')
  }
  return null
}

function summarizeMockArtifacts(artifacts: ImportJob['artifacts']) {
  const totals = new Map<
    ImportJob['artifacts'][number]['artifactKind'],
    { count: number; totalBytes: number }
  >()
  for (const artifact of artifacts) {
    const current = totals.get(artifact.artifactKind) ?? { count: 0, totalBytes: 0 }
    current.count += 1
    current.totalBytes += artifact.sizeBytes
    totals.set(artifact.artifactKind, current)
  }
  return [...totals].map(([kind, value]) => ({ kind, ...value }))
}

registerMock('GET', '/api/v1/import-jobs/:id', ({ path, headers }) => {
  requireMember(headers)
  const id = decodeURIComponent(path.split('/').at(-1) ?? '')
  const job = mockImportJobs[id]
  if (!job) throw new ApiError('导入任务不存在', 404, path)
  return job
})

registerMock('POST', '/api/v1/import-jobs/:id/cancel', ({ path, headers }) => {
  requireMember(headers)
  const id = decodeURIComponent(path.split('/').at(-2) ?? '')
  const job = mockImportJobs[id]
  if (!job) throw new ApiError('导入任务不存在', 404, path)
  if (['completed', 'failed', 'cancelled'].includes(job.status)) {
    throw new ApiError('该整理任务已经结束', 409, path)
  }
  job.status = 'cancelled'
  job.stage = '已取消'
  job.updatedAt = new Date().toISOString()
  return job
})

registerMock('POST', '/api/v1/import-jobs/:id/refine', ({ path, headers, body }) => {
  requireMember(headers)
  const id = decodeURIComponent(path.split('/').at(-2) ?? '')
  const job = mockImportJobs[id]
  if (!job) throw new ApiError('导入任务不存在', 404, path)
  if (job.status !== 'completed') {
    throw new ApiError('请等待整理流程完成后再提交补充提示', 409, path)
  }
  const prompt = (body as { prompt?: unknown })?.prompt
  if (typeof prompt !== 'string' || !prompt.trim()) {
    throw new ApiError('补充提示词不能为空', 400, path)
  }
  job.inputs = [
    ...job.inputs.filter((input) => input.displayName !== '整理补充提示'),
    {
      id: crypto.randomUUID(), inputKind: 'prompt', provider: 'user',
      displayName: '整理补充提示', sourceRef: prompt.trim(), mimeType: null,
      sizeBytes: null, status: 'queued_codex',
    },
  ]
  job.stage = '补充提示已保存，等待 Codex'
  job.updatedAt = new Date().toISOString()
  return job
})

registerMock('POST', '/api/v1/projects/:slug/cover/generate', ({ path, headers }) => {
  requireMember(headers)
  const slug = decodeURIComponent(path.split('/').at(-3) ?? '')
  const project = mockProjects.find((item) => item.slug === slug)
  if (!project) throw new ApiError('项目不存在', 404, path)
  const generated = buildCoverPreview(project)
  Object.assign(project, generated)
  return generated
})

registerMock('PATCH', '/api/v1/projects/:slug/cover', ({ path, headers, body }) => {
  requireMember(headers)
  const slug = decodeURIComponent(path.split('/').at(-2) ?? '')
  const project = mockProjects.find((item) => item.slug === slug)
  if (!project) throw new ApiError('项目不存在', 404, path)
  const input = body as Pick<ProjectDetail, 'coverMode' | 'coverResourceId' | 'coverTitle' | 'coverSubtitle' | 'coverKeywords' | 'coverTone'>
  Object.assign(project, input, { coverResourceUrl: null, coverConfidence: 1 })
  return { ...input, coverResourceUrl: null, coverConfidence: 1 }
})

import { ApiError, registerMock } from '@/api/client'
import { PROJECT_FIXTURES } from '@/api/mock/fixtures'
import { LoginRequestSchema, type User } from '@/api/schemas/user'
import {
  ProjectCategorySchema,
  ProjectWriteInputSchema,
  type ProjectDetail,
  type ProjectWriteInput,
} from '@/api/schemas/project'
import { filterProjects } from '@/lib/projects'

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

function requireManager(headers: Headers): User {
  const token = headers.get('Authorization')?.replace(/^Bearer\s+/i, '')
  const sid = token?.startsWith('mock:') ? token.slice(5) : null
  if (!sid) throw new ApiError('请先登录', 401, '/api/v1/projects')
  const user = mockUser(sid)
  if (!user.isLabMember && !user.isSuperAdmin) {
    throw new ApiError('只有实验室成员可以管理项目', 403, '/api/v1/projects')
  }
  return user
}

function detailFromInput(input: ProjectWriteInput, id: string = crypto.randomUUID()): ProjectDetail {
  return {
    id,
    ...input,
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

registerMock('GET', '/api/v1/projects', ({ query }) => {
  const categoryRaw = query.get('category')
  const category = categoryRaw ? ProjectCategorySchema.parse(categoryRaw) : undefined
  const items = filterProjects(mockProjects, query.get('q') ?? undefined, category)
  return { items, total: items.length }
})

registerMock('GET', '/api/v1/projects/:slug', ({ path }) => {
  const slug = decodeURIComponent(path.split('/').at(-1) ?? '')
  const project = mockProjects.find((item) => item.slug === slug)
  if (!project) throw new ApiError('项目不存在', 404, path)
  return project
})

registerMock('POST', '/api/v1/projects', ({ body, headers }) => {
  requireManager(headers)
  const input = ProjectWriteInputSchema.parse(body)
  if (mockProjects.some((project) => project.slug === input.slug)) {
    throw new ApiError(`项目路径 ${input.slug} 已存在`, 409, '/api/v1/projects')
  }
  const project = detailFromInput(input)
  mockProjects = [project, ...mockProjects]
  return project
})

registerMock('PUT', '/api/v1/projects/:slug', ({ body, headers, path }) => {
  requireManager(headers)
  const currentSlug = decodeURIComponent(path.split('/').at(-1) ?? '')
  const index = mockProjects.findIndex((project) => project.slug === currentSlug)
  if (index < 0) throw new ApiError('项目不存在', 404, path)
  const input = ProjectWriteInputSchema.parse(body)
  if (mockProjects.some((project, itemIndex) => project.slug === input.slug && itemIndex !== index)) {
    throw new ApiError(`项目路径 ${input.slug} 已存在`, 409, path)
  }
  const project = detailFromInput(input, mockProjects[index]!.id)
  mockProjects[index] = project
  return project
})

registerMock('DELETE', '/api/v1/projects/:slug', ({ headers, path }) => {
  requireManager(headers)
  const slug = decodeURIComponent(path.split('/').at(-1) ?? '')
  const index = mockProjects.findIndex((project) => project.slug === slug)
  if (index < 0) throw new ApiError('项目不存在', 404, path)
  mockProjects.splice(index, 1)
  return null
})

registerMock('POST', '/api/v1/projects/import', ({ body, headers }) => {
  requireManager(headers)
  const payload = ProjectWriteInputSchema.array().max(200).parse(
    (body as { items?: unknown } | null)?.items,
  )
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

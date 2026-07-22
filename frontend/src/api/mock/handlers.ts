import { ApiError, registerMock } from '@/api/client'
import { PROJECT_FIXTURES } from '@/api/mock/fixtures'
import { LoginRequestSchema, type User } from '@/api/schemas/user'
import { ProjectCategorySchema } from '@/api/schemas/project'
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
  const items = filterProjects(PROJECT_FIXTURES, query.get('q') ?? undefined, category)
  return { items, total: items.length }
})

registerMock('GET', '/api/v1/projects/:slug', ({ path }) => {
  const slug = decodeURIComponent(path.split('/').at(-1) ?? '')
  const project = PROJECT_FIXTURES.find((item) => item.slug === slug)
  if (!project) throw new ApiError('项目不存在', 404, path)
  return project
})

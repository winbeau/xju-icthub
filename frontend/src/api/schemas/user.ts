import { z } from 'zod'

export const UserRoleSchema = z.enum(['user', 'admin', 'superadmin'])

export const UserSchema = z.object({
  sid: z.string(),
  name: z.string(),
  nickname: z.string(),
  preferredName: z.string().nullish(),
  avatar: z.string().url().nullish(),
  avatarThumb: z.string().url().nullish(),
  bio: z.string().nullish(),
  wechat: z.string().nullish(),
  phone: z.string().nullish(),
  email: z.string().nullish(),
  role: UserRoleSchema.nullish(),
  isAdmin: z.boolean().nullish(),
  isSuperAdmin: z.boolean().nullish(),
  isLabMember: z.boolean().nullish(),
  classId: z.number().nullish(),
  classFullName: z.string().nullish(),
  classShortName: z.string().nullish(),
  isClassCommittee: z.boolean().nullish(),
  committeeTitle: z.string().nullish(),
})

export type User = z.infer<typeof UserSchema>

export const LoginResponseSchema = z.object({
  user: UserSchema,
  token: z.string().min(1),
})

export type LoginResponse = z.infer<typeof LoginResponseSchema>

export const LoginRequestSchema = z.object({
  sid: z.string().regex(/^\d{11}$/, '学号需 11 位纯数字'),
  password: z.string().min(1, '密码不能为空'),
})

export type LoginRequest = z.infer<typeof LoginRequestSchema>

export function canAccessIctHub(user: User | null): boolean {
  return Boolean(user?.isLabMember || user?.isSuperAdmin || user?.role === 'superadmin')
}

export function canManageProjects(user: User | null): boolean {
  return canAccessIctHub(user)
}

export function canManageTags(user: User | null): boolean {
  return Boolean(user?.isAdmin || user?.isSuperAdmin || user?.role === 'admin' || user?.role === 'superadmin')
}

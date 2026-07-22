import { describe, expect, it } from 'vitest'
import { UserSchema, canManageProjects } from '@/api/schemas/user'

const baseUser = {
  sid: '20211010000',
  name: '测试用户',
  nickname: '测试用户',
}

describe('UserSchema', () => {
  it('keeps compatibility before the Feiyue lab-member migration', () => {
    const user = UserSchema.parse(baseUser)
    expect(user.isLabMember ?? false).toBe(false)
    expect(canManageProjects(user)).toBe(false)
  })

  it('allows lab members and superadmins to manage projects', () => {
    const member = UserSchema.parse({ ...baseUser, isLabMember: true })
    const superadmin = UserSchema.parse({ ...baseUser, role: 'superadmin' })
    expect(canManageProjects(member)).toBe(true)
    expect(canManageProjects(superadmin)).toBe(true)
  })
})

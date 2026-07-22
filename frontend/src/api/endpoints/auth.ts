import { z } from 'zod'
import { request } from '@/api/client'
import { LoginResponseSchema, UserSchema, type LoginResponse, type User } from '@/api/schemas/user'

export const TOKEN_KEY = 'icthub.auth.token'

export function authHeaders(): Record<string, string> {
  const token = localStorage.getItem(TOKEN_KEY)
  return token ? { Authorization: `Bearer ${token}` } : {}
}

export function login(sid: string, password: string): Promise<LoginResponse> {
  return request({
    method: 'POST',
    path: '/auth/login',
    body: { sid, password },
    schema: LoginResponseSchema,
  })
}

export function me(token: string): Promise<User> {
  return request({
    method: 'GET',
    path: '/auth/me',
    schema: UserSchema,
    headers: { Authorization: `Bearer ${token}` },
  })
}

export async function logout(): Promise<void> {
  await request({
    method: 'POST',
    path: '/auth/logout',
    schema: z.null(),
    headers: authHeaders(),
  })
}

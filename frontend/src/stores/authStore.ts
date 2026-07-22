import { create, type StateCreator } from 'zustand'
import { createJSONStorage, persist } from 'zustand/middleware'
import * as authApi from '@/api/endpoints/auth'
import type { User } from '@/api/schemas/user'

export type AuthMode = 'authed' | 'guest' | 'anon'

export type AuthState = {
  user: User | null
  token: string | null
  mode: AuthMode
  login: (sid: string, password: string) => Promise<User>
  logout: () => void
  enterAsGuest: () => void
  hydrateFromToken: () => Promise<void>
}

const creator: StateCreator<AuthState, [['zustand/persist', unknown]]> = (set, get) => ({
  user: null,
  token: null,
  mode: 'anon',
  login: async (sid, password) => {
    const { user, token } = await authApi.login(sid, password)
    localStorage.setItem(authApi.TOKEN_KEY, token)
    set({ user, token, mode: 'authed' })
    return user
  },
  logout: () => {
    void authApi.logout().catch(() => undefined)
    localStorage.removeItem(authApi.TOKEN_KEY)
    set({ user: null, token: null, mode: 'anon' })
  },
  enterAsGuest: () => set({ user: null, token: null, mode: 'guest' }),
  hydrateFromToken: async () => {
    const token = get().token ?? localStorage.getItem(authApi.TOKEN_KEY)
    if (!token) return
    try {
      const user = await authApi.me(token)
      set({ user, token, mode: 'authed' })
    } catch {
      localStorage.removeItem(authApi.TOKEN_KEY)
      set({ user: null, token: null, mode: 'anon' })
    }
  },
})

export const useAuthStore = create<AuthState>()(
  persist(creator, {
    name: 'icthub.auth',
    storage: createJSONStorage(() => localStorage),
    partialize: (state) => ({ user: state.user, token: state.token, mode: state.mode }),
  }),
)

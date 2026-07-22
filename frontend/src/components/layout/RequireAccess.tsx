import type { ReactNode } from 'react'
import { Navigate, useLocation } from 'react-router-dom'
import { canAccessIctHub } from '@/api/schemas/user'
import { AccessDeniedPage } from '@/pages/AccessDeniedPage'
import { useAuthStore } from '@/stores/authStore'

type RequireAccessProps = {
  children: ReactNode
  requireLabMember?: boolean
}

export function RequireAccess({ children, requireLabMember = false }: RequireAccessProps) {
  const user = useAuthStore((state) => state.user)
  const mode = useAuthStore((state) => state.mode)
  const location = useLocation()

  if (mode !== 'authed') {
    return <Navigate to="/login" replace state={{ from: location.pathname }} />
  }

  if (requireLabMember && !canAccessIctHub(user)) {
    return <AccessDeniedPage />
  }

  return <>{children}</>
}

import type { ReactNode } from 'react'
import { Navigate, useLocation } from 'react-router-dom'
import { canManageProjects } from '@/api/schemas/user'
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

  if (requireLabMember && !canManageProjects(user)) {
    return <Navigate to="/projects" replace />
  }

  return <>{children}</>
}

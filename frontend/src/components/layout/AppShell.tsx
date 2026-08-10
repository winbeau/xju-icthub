import { LogIn, LogOut, Plus } from 'lucide-react'
import { Link, Outlet } from 'react-router-dom'
import { canManageProjects } from '@/api/schemas/user'
import { Button } from '@/components/ui/button'
import { useAuthStore } from '@/stores/authStore'

export function AppShell() {
  const user = useAuthStore((state) => state.user)
  const mode = useAuthStore((state) => state.mode)
  const logout = useAuthStore((state) => state.logout)
  const canManage = canManageProjects(user)
  const currentYear = new Date().getFullYear()

  return (
    <div className="flex min-h-screen flex-col bg-bg text-text">
      <header className="border-b border-border bg-bg/95">
        <div className="mx-auto flex h-16 max-w-6xl items-center gap-5 px-5 sm:px-8">
          <Link
            to="/"
            className="shrink-0 font-serif text-lg font-semibold tracking-[-0.01em] text-text"
          >
            Winbeau
          </Link>

          <nav
            aria-label="主导航"
            className="hidden min-w-0 flex-1 items-center gap-5 text-sm text-text-muted sm:flex"
          >
            <Link className="transition-colors hover:text-text" to="/">
              首页
            </Link>
            <Link className="transition-colors hover:text-text" to="/projects">
              项目
            </Link>
          </nav>

          <div className="flex shrink-0 items-center gap-2">
            {canManage && (
              <Button asChild size="sm" className="hidden sm:inline-flex">
                <Link to="/admin/projects/new">
                  <Plus aria-hidden />
                  上传项目
                </Link>
              </Button>
            )}

            {mode === 'authed' && user ? (
              <>
                <span className="hidden max-w-32 truncate text-sm text-text-muted md:inline">
                  {user.nickname}
                </span>
                <Button variant="ghost" size="sm" onClick={logout} aria-label="退出登录">
                  <LogOut aria-hidden />
                  <span className="hidden sm:inline">退出</span>
                </Button>
              </>
            ) : (
              <Button asChild variant="ghost" size="sm">
                <Link to="/login">
                  <LogIn aria-hidden />
                  登录
                </Link>
              </Button>
            )}
          </div>
        </div>
      </header>

      <main className="flex-1">
        <Outlet />
      </main>

      <footer className="border-t border-border">
        <div className="mx-auto flex max-w-6xl flex-col gap-1 px-5 py-7 text-sm text-text-faint sm:flex-row sm:items-center sm:justify-between sm:px-8">
          <span>© {currentYear} Winbeau</span>
          <span>Personal website · icthub.top</span>
        </div>
      </footer>
    </div>
  )
}

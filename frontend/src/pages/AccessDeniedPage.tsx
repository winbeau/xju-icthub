import { LogOut, ShieldX } from 'lucide-react'
import { useNavigate } from 'react-router-dom'
import { Button } from '@/components/ui/button'
import { useAuthStore } from '@/stores/authStore'

export function AccessDeniedPage() {
  const navigate = useNavigate()
  const user = useAuthStore((state) => state.user)
  const logout = useAuthStore((state) => state.logout)

  const switchAccount = () => {
    logout()
    navigate('/login', { replace: true })
  }

  return (
    <main className="flex min-h-screen items-center justify-center bg-bg px-6 py-16 text-text">
      <section className="w-full max-w-xl rounded-lg border border-border bg-bg p-8 text-center shadow-card sm:p-12">
        <ShieldX className="mx-auto size-10 text-text-muted" strokeWidth={1.6} aria-hidden />
        <p className="mt-6 text-sm font-medium uppercase tracking-[0.16em] text-text-faint">
          Winbeau / Restricted
        </p>
        <h1 className="mt-3 font-serif text-3xl font-semibold tracking-[-0.02em]">暂无访问权限</h1>
        <p className="mx-auto mt-4 max-w-md text-lg leading-8 text-text-muted">
          当前账号尚未开通项目库权限，请联系管理员
        </p>
        {user && (
          <p className="mt-3 text-sm text-text-faint">
            当前账号：{user.nickname}（{user.sid}）
          </p>
        )}
        <Button className="mt-8" variant="outline" onClick={switchAccount}>
          <LogOut aria-hidden />
          退出并更换账号
        </Button>
      </section>
    </main>
  )
}

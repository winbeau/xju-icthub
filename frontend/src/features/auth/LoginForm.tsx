import { useEffect } from 'react'
import { zodResolver } from '@hookform/resolvers/zod'
import { useForm } from 'react-hook-form'
import { useLocation, useNavigate } from 'react-router-dom'
import { toast } from 'sonner'
import { ApiError } from '@/api/client'
import { LoginRequestSchema, type LoginRequest } from '@/api/schemas/user'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { Label } from '@/components/ui/label'
import { useAuthStore } from '@/stores/authStore'

export function LoginForm() {
  const navigate = useNavigate()
  const location = useLocation()
  const login = useAuthStore((state) => state.login)
  const enterAsGuest = useAuthStore((state) => state.enterAsGuest)
  const mode = useAuthStore((state) => state.mode)

  const {
    register,
    handleSubmit,
    setError,
    formState: { errors, isSubmitting },
  } = useForm<LoginRequest>({
    resolver: zodResolver(LoginRequestSchema),
    defaultValues: { sid: '', password: '' },
    mode: 'onTouched',
  })

  const from = (location.state as { from?: string } | null)?.from ?? '/projects'

  useEffect(() => {
    if (mode === 'authed') navigate(from, { replace: true })
  }, [from, mode, navigate])

  const onSubmit = handleSubmit(async (values) => {
    try {
      await login(values.sid, values.password)
      toast.success('登录成功')
      navigate(from, { replace: true })
    } catch (error) {
      const message = error instanceof ApiError ? error.message : '登录失败，请稍后再试'
      toast.error(message)
      setError('password', { type: 'server', message })
    }
  })

  const onGuest = () => {
    enterAsGuest()
    navigate('/projects', { replace: true })
  }

  return (
    <div className="w-full max-w-[408px] rounded-lg border border-border bg-bg p-8 shadow-card sm:p-10">
      <p className="text-xs font-medium uppercase tracking-[0.16em] text-text-faint">ICTHub</p>
      <h1 className="mt-3 font-serif text-[32px] font-semibold leading-[1.15] text-text">登录</h1>
      <p className="mt-2 text-[15px] leading-[1.6] text-text-muted">
        使用已有飞跃账号进入实验室项目库。
      </p>

      <form className="mt-8 space-y-4" noValidate onSubmit={onSubmit}>
        <div className="space-y-1.5">
          <Label htmlFor="login-sid" className="text-[13px]">
            学号
          </Label>
          <Input
            id="login-sid"
            type="text"
            inputMode="numeric"
            autoComplete="username"
            placeholder="20211010001"
            maxLength={11}
            className="h-11 text-base"
            aria-invalid={Boolean(errors.sid)}
            {...register('sid')}
          />
          {errors.sid && <p className="text-xs text-cat-internet">{errors.sid.message}</p>}
        </div>

        <div className="space-y-1.5">
          <Label htmlFor="login-password" className="text-[13px]">
            密码
          </Label>
          <Input
            id="login-password"
            type="password"
            autoComplete="current-password"
            placeholder="••••••"
            className="h-11 text-base"
            aria-invalid={Boolean(errors.password)}
            {...register('password')}
          />
          {errors.password && (
            <p role="alert" className="text-xs text-cat-internet">
              {errors.password.message}
            </p>
          )}
        </div>

        <Button type="submit" size="lg" className="h-11 w-full" disabled={isSubmitting}>
          {isSubmitting ? '登录中…' : '登录'}
        </Button>
      </form>

      <div className="my-5 flex items-center gap-3 text-xs text-text-faint">
        <span className="h-px flex-1 bg-border" />
        <span>或</span>
        <span className="h-px flex-1 bg-border" />
      </div>

      <Button type="button" variant="outline" size="lg" className="h-11 w-full" onClick={onGuest}>
        以游客身份浏览
      </Button>

      {import.meta.env.DEV && (
        <div className="mt-6 rounded-md bg-bg-subtle px-3 py-2.5 text-[12px] leading-5 text-text-muted">
          Mock 成员：<span className="font-mono">20211010001 / 123456</span>
          <br />
          Mock 非成员：<span className="font-mono">20211010000 / 123456</span>
        </div>
      )}
    </div>
  )
}

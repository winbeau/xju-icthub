import { BrandPanel } from '@/features/auth/BrandPanel'
import { LoginForm } from '@/features/auth/LoginForm'

export function LoginPage() {
  return (
    <main className="grid min-h-screen grid-cols-1 lg:grid-cols-12">
      <BrandPanel />
      <section className="flex items-center justify-center bg-bg px-6 py-12 lg:col-span-5">
        <LoginForm />
      </section>
    </main>
  )
}

import { Construction } from 'lucide-react'

export function AdminProjectsPage() {
  return (
    <div className="mx-auto max-w-4xl px-5 py-16 sm:px-8">
      <div className="rounded-lg border border-border bg-bg-subtle p-8">
        <Construction className="text-text-muted" aria-hidden />
        <h1 className="mt-5 font-serif text-3xl font-semibold">项目管理工作台</h1>
        <p className="mt-3 max-w-2xl leading-7 text-text-muted">
          登录与实验室成员守卫已经接入。下一步在这里实现项目创建、编辑、资源维护和快速导入。
        </p>
      </div>
    </div>
  )
}

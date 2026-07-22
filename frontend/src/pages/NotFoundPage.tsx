import { Link } from 'react-router-dom'

export function NotFoundPage() {
  return (
    <div className="mx-auto max-w-4xl px-5 py-24 text-center sm:px-8">
      <p className="font-mono text-sm text-text-faint">404</p>
      <h1 className="mt-4 font-serif text-3xl font-semibold">页面不存在</h1>
      <Link to="/projects" className="mt-6 inline-block text-sm text-link hover:underline">
        返回项目库
      </Link>
    </div>
  )
}

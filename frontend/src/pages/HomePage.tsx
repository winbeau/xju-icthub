import { ArrowRight } from 'lucide-react'
import { Link } from 'react-router-dom'

export function HomePage() {
  return (
    <div className="mx-auto flex min-h-[calc(100vh-130px)] max-w-6xl items-center px-5 py-16 sm:px-8">
      <section className="max-w-3xl">
        <p className="text-xs font-medium uppercase tracking-[0.18em] text-text-faint">
          XINJIANG UNIVERSITY · ICT &amp; SOFTWARE
        </p>
        <h1 className="mt-5 font-serif text-4xl font-semibold leading-tight tracking-[-0.025em] sm:text-6xl">
          新疆大学 ICT&amp;软开实验室
        </h1>
        <p className="mt-6 max-w-2xl text-base leading-8 text-text-muted sm:text-lg">
          主页内容正在建设。首期先开放项目库，把历届与在研项目整理成可以检索、交接和继续利用的实验室资产。
        </p>
        <Link
          to="/projects"
          className="mt-8 inline-flex items-center gap-2 border-b border-text pb-1 text-sm font-medium text-text"
        >
          进入项目库
          <ArrowRight size={16} aria-hidden />
        </Link>
      </section>
    </div>
  )
}

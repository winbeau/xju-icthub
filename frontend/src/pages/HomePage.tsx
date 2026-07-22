import { ArrowRight } from 'lucide-react'
import { Link } from 'react-router-dom'

export function HomePage() {
  return (
    <div className="mx-auto flex min-h-[calc(100vh-130px)] max-w-6xl items-center px-5 py-16 sm:px-8">
      <section className="max-w-3xl">
        <p className="text-sm font-medium uppercase tracking-[0.18em] text-text-faint">
          XINJIANG UNIVERSITY · ICT &amp; SOFTWARE
        </p>
        <h1 className="mt-5 font-serif text-4xl font-semibold leading-tight tracking-[-0.025em] sm:text-6xl">
          新疆大学 ICT&amp;软开实验室
        </h1>
        <p className="mt-6 max-w-2xl text-lg leading-8 text-text-muted sm:text-xl">
          记录实验室的探索、实践与成果。
        </p>
        <Link
          to="/projects"
          className="mt-8 inline-flex items-center gap-2 border-b border-text pb-1 text-base font-medium text-text"
        >
          进入项目库
          <ArrowRight size={16} aria-hidden />
        </Link>
      </section>
    </div>
  )
}

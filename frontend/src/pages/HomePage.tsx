import { ArrowRight, Github } from 'lucide-react'
import { Link } from 'react-router-dom'

const focusAreas = [
  {
    label: 'BUILD',
    title: 'AI 与自动化',
    description: '把重复工作交给可靠的工具，让人专注于判断、创造和推进。',
  },
  {
    label: 'SHIP',
    title: 'Web 产品',
    description: '从真实需求出发，持续打磨能够稳定运行、方便使用的产品。',
  },
  {
    label: 'LEARN',
    title: '技术实践',
    description: '记录开发过程中的探索、取舍、复盘，以及仍在生长的想法。',
  },
] as const

export function HomePage() {
  return (
    <div className="mx-auto max-w-6xl px-5 py-16 sm:px-8 sm:py-24">
      <section className="grid gap-12 border-b border-border pb-16 lg:grid-cols-[minmax(0,1.45fr)_minmax(280px,0.7fr)] lg:items-end lg:gap-20">
        <div>
          <p className="text-sm font-medium uppercase tracking-[0.18em] text-text-faint">
            WINBEAU · PERSONAL WEBSITE
          </p>
          <h1 className="mt-5 max-w-4xl font-serif text-5xl font-semibold leading-[1.05] tracking-[-0.035em] sm:text-7xl">
            你好，我是 Winbeau。
          </h1>
          <p className="mt-7 max-w-2xl text-lg leading-8 text-text-muted sm:text-xl">
            我是一名开发者，关注 AI 工具、Web 产品与自动化。这里记录我正在构建的项目、技术实践和持续迭代。
          </p>
          <div className="mt-9 flex flex-wrap items-center gap-5">
            <Link
              to="/projects"
              className="inline-flex items-center gap-2 border-b border-text pb-1 text-base font-medium text-text"
            >
              浏览我的项目
              <ArrowRight size={16} aria-hidden />
            </Link>
            <a
              href="https://github.com/winbeau"
              target="_blank"
              rel="noreferrer"
              className="inline-flex items-center gap-2 text-base text-text-muted transition-colors hover:text-text"
            >
              <Github size={17} aria-hidden />
              GitHub
            </a>
          </div>
        </div>

        <aside className="border-l border-border pl-6 sm:pl-8">
          <p className="text-xs font-medium uppercase tracking-[0.16em] text-text-faint">CURRENTLY</p>
          <h2 className="mt-3 font-serif text-2xl font-semibold tracking-[-0.02em]">
            把想法做成可用的产品。
          </h2>
          <p className="mt-4 text-base leading-7 text-text-muted">
            从原型到部署，保持简单、快速和可维护。
          </p>
        </aside>
      </section>

      <section className="grid gap-8 py-12 sm:grid-cols-3 sm:py-16" aria-label="关注方向">
        {focusAreas.map((area) => (
          <article key={area.label}>
            <p className="text-xs font-medium tracking-[0.16em] text-text-faint">{area.label}</p>
            <h2 className="mt-3 font-serif text-2xl font-semibold tracking-[-0.02em]">
              {area.title}
            </h2>
            <p className="mt-3 text-base leading-7 text-text-muted">{area.description}</p>
          </article>
        ))}
      </section>
    </div>
  )
}

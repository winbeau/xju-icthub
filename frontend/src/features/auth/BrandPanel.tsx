import type { CSSProperties } from 'react'
import { ArrowUpRight, FolderKanban, Trophy } from 'lucide-react'
import { Link } from 'react-router-dom'
import { PROJECT_FIXTURES } from '@/api/mock/fixtures'
import { categoryColor } from '@/lib/projects'

const positions = [
  { x: '-70%', y: '0px', rot: '-4deg' },
  { x: '-50%', y: '24px', rot: '2deg' },
  { x: '-30%', y: '48px', rot: '-1deg' },
] as const

const backgroundStyle: CSSProperties = {
  backgroundColor: 'var(--brand-panel-bg)',
  backgroundImage: [
    'radial-gradient(at 12% 18%, var(--brand-panel-glow-warm), transparent 55%)',
    'radial-gradient(at 82% 82%, var(--brand-panel-glow-cool), transparent 60%)',
    'radial-gradient(var(--brand-panel-grid) 1px, transparent 1px)',
  ].join(', '),
  backgroundSize: 'auto, auto, 24px 24px',
}

export function BrandPanel() {
  return (
    <aside
      className="relative hidden select-none overflow-hidden lg:col-span-7 lg:flex lg:flex-col lg:justify-center lg:px-20 lg:py-16 xl:px-28"
      style={backgroundStyle}
      aria-label="Winbeau 项目库简介"
    >
      <header>
        <Link
          to="/projects"
          className="inline-flex items-center gap-2 font-serif text-lg font-semibold"
        >
          <FolderKanban size={20} strokeWidth={1.75} aria-hidden />
          Winbeau / Projects
        </Link>
        <h2 className="mt-9 max-w-[14ch] font-serif text-[46px] font-semibold leading-[1.05] tracking-[-0.025em] xl:text-[56px]">
          把想法，
          <br />
          做成作品。
        </h2>
        <p className="mt-4 max-w-[42ch] text-base leading-7 text-text-muted">
          我的项目、实验与长期积累。
        </p>
      </header>

      <div className="relative mx-auto mt-8 h-[224px] w-full max-w-[620px]" aria-hidden>
        {PROJECT_FIXTURES.slice(0, 3).map((project, index) => {
          const position = positions[index] ?? positions[1]
          const style = {
            '--card-x': position.x,
            '--card-y': position.y,
            '--card-rot': position.rot,
            zIndex: index + 1,
            animationDelay: `${index * 120}ms`,
          } as CSSProperties
          return (
            <article
              key={project.id}
              className="project-stack-card absolute left-1/2 top-0 w-[410px] rounded-lg border border-border bg-bg p-5 shadow-card"
              style={style}
            >
              <span
                className="text-xs font-medium"
                style={{ color: categoryColor(project.primaryCategory) }}
              >
                {project.primaryCategory}
              </span>
              <h3 className="mt-2 font-serif text-lg font-semibold">{project.name}</h3>
              <p className="mt-1 line-clamp-2 text-[13px] leading-5 text-text-muted">
                {project.summary}
              </p>
              <div className="mt-3 flex items-center gap-1.5 text-xs text-text-faint">
                <Trophy size={13} aria-hidden />
                <span>{project.highestAward ?? '暂无奖项'}</span>
                <ArrowUpRight size={13} className="ml-auto" aria-hidden />
              </div>
            </article>
          )
        })}
      </div>

      <p className="mt-7 text-xs text-text-faint">个人账号 · 独立项目数据</p>
    </aside>
  )
}

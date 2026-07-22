import { useDeferredValue, useState } from 'react'
import { useQuery } from '@tanstack/react-query'
import { ArrowRight, Search } from 'lucide-react'
import { Link } from 'react-router-dom'
import { listProjects } from '@/api/endpoints/projects'
import type { ProjectCategory } from '@/api/schemas/project'
import { ProjectCover } from '@/components/projects/ProjectCover'
import { Input } from '@/components/ui/input'
import { PROJECT_CATEGORIES, categoryColor } from '@/lib/projects'

export function ProjectsPage() {
  const [search, setSearch] = useState('')
  const [category, setCategory] = useState<ProjectCategory | undefined>()
  const deferredSearch = useDeferredValue(search)
  const projects = useQuery({
    queryKey: ['projects', deferredSearch, category],
    queryFn: () => listProjects({ q: deferredSearch || undefined, category }),
  })

  return (
    <div className="mx-auto max-w-6xl px-5 py-12 sm:px-8 sm:py-16">
      <section className="grid gap-8 border-b border-border pb-9 md:grid-cols-[minmax(0,1fr)_minmax(280px,380px)] md:items-end">
        <div className="min-w-0">
          <p className="text-sm font-medium uppercase tracking-[0.14em] text-text-faint">
            ICTHub / Projects
          </p>
          <h1 className="mt-3 font-serif text-4xl font-semibold tracking-[-0.03em] sm:text-5xl">
            实验室项目集
          </h1>
          <p className="mt-4 text-lg text-text-muted">让想法、作品与研究持续生长。</p>
        </div>
        <label className="block w-full min-w-0">
          <span className="mb-2 block text-base font-medium text-text-muted">搜索项目</span>
          <span className="relative block">
            <Search
              size={17}
              className="pointer-events-none absolute left-3 top-1/2 -translate-y-1/2 text-text-faint"
              aria-hidden
            />
            <Input
              value={search}
              onChange={(event) => setSearch(event.target.value)}
              placeholder="项目名称、简介或获奖"
              className="h-11 pl-10"
            />
          </span>
        </label>
      </section>

      <div className="flex flex-wrap gap-x-6 gap-y-3 py-6" aria-label="项目类别">
        <CategoryButton active={!category} onClick={() => setCategory(undefined)}>
          全部
        </CategoryButton>
        {PROJECT_CATEGORIES.map((item) => (
          <CategoryButton key={item} active={category === item} onClick={() => setCategory(item)}>
            {item}
          </CategoryButton>
        ))}
      </div>

      <section
        className="grid min-h-72 gap-7 sm:grid-cols-2 lg:grid-cols-3"
        aria-label="项目目录"
        aria-busy={projects.isLoading}
      >
        {projects.isLoading &&
          [1, 2, 3, 4, 5, 6].map((item) => (
            <div key={item} className="animate-pulse">
              <div className="aspect-[16/10] rounded-lg bg-bg-subtle" />
              <div className="mt-4 h-6 w-3/5 rounded bg-bg-subtle" />
              <div className="mt-3 flex justify-between gap-3">
                <div className="h-6 w-24 rounded-full bg-bg-subtle" />
                <div className="h-6 w-32 rounded-full bg-bg-subtle" />
              </div>
            </div>
          ))}

        {projects.isError && (
          <p className="col-span-full border-y border-border py-12 text-base text-cat-internet">
            项目加载失败，请稍后重试。
          </p>
        )}

        {projects.data?.items.map((project) => (
          <Link
            key={project.id}
            to={`/projects/${project.slug}`}
            className="group min-w-0 overflow-hidden rounded-xl border border-border bg-bg transition-[border-color,box-shadow,transform] duration-200 hover:-translate-y-1 hover:border-border-strong hover:shadow-lg"
          >
            <ProjectCover
              cover={project}
              className="aspect-[4/3] rounded-none transition-transform duration-300 group-hover:scale-[1.015]"
            />
            <div className="px-4 pb-4 pt-4">
              <div className="flex items-start justify-between gap-3">
                <h2 className="relative min-w-0 flex-1 overflow-hidden whitespace-nowrap pr-10 font-serif text-2xl font-semibold leading-tight tracking-[-0.02em] after:pointer-events-none after:absolute after:inset-y-0 after:right-0 after:w-10 after:bg-gradient-to-r after:from-transparent after:to-bg">
                  {project.name}
                </h2>
                <ArrowRight
                  size={17}
                  className="mt-1 shrink-0 text-text-faint transition-transform group-hover:translate-x-1"
                  aria-hidden
                />
              </div>
              <div className="mt-3 flex min-h-7 items-center gap-3">
                <span
                  className="rounded-full bg-bg-subtle px-2.5 py-1 text-sm font-medium"
                  style={{ color: categoryColor(project.primaryCategory) }}
                >
                  {project.primaryCategory}
                </span>
                {competitionTag(project.tags) && (
                  <span className="ml-auto max-w-[65%] truncate rounded-full border border-border px-2.5 py-1 text-sm text-text-muted">
                    {competitionTag(project.tags)}
                  </span>
                )}
              </div>
            </div>
          </Link>
        ))}

        {projects.data && projects.data.items.length === 0 && (
          <p className="col-span-full border-y border-border py-14 text-base text-text-muted">
            没有匹配项目。
          </p>
        )}
      </section>
    </div>
  )
}

function competitionTag(tags: readonly string[]): string | undefined {
  const competitions = ['国创赛（互联网+）', '计算机设计大赛', '智能应用技术大赛']
  return competitions.find((competition) => tags.includes(competition))
}

function CategoryButton({
  active,
  children,
  onClick,
}: {
  active: boolean
  children: string
  onClick: () => void
}) {
  return (
    <button
      type="button"
      aria-pressed={active}
      onClick={onClick}
      className={`border-b pb-1 text-base transition-colors ${
        active ? 'border-text font-semibold text-text' : 'border-transparent text-text-muted hover:text-text'
      }`}
    >
      {children}
    </button>
  )
}

import { useDeferredValue, useState } from 'react'
import { useQuery } from '@tanstack/react-query'
import { ArrowRight, Search, Trophy } from 'lucide-react'
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
              <div className="mt-3 h-4 w-full rounded bg-bg-subtle" />
            </div>
          ))}

        {projects.isError && (
          <p className="col-span-full border-y border-border py-12 text-base text-cat-internet">
            项目加载失败，请稍后重试。
          </p>
        )}

        {projects.data?.items.map((project) => (
          <Link key={project.id} to={`/projects/${project.slug}`} className="group min-w-0">
            <ProjectCover
              cover={project}
              className="aspect-[16/10] transition-transform duration-200 group-hover:-translate-y-1"
            />
            <div className="pt-4">
              <div className="flex items-center justify-between gap-3">
                <span
                  className="text-sm font-semibold"
                  style={{ color: categoryColor(project.primaryCategory) }}
                >
                  {project.primaryCategory}
                </span>
                <ArrowRight
                  size={17}
                  className="text-text-faint transition-transform group-hover:translate-x-1"
                  aria-hidden
                />
              </div>
              <h2 className="mt-2 font-serif text-2xl font-semibold leading-tight tracking-[-0.02em]">
                {project.name}
              </h2>
              <p className="mt-2 line-clamp-2 leading-7 text-text-muted">{project.summary}</p>
              <div className="mt-3 flex flex-wrap gap-1.5">
                {project.tags.slice(0, 3).map((tag) => (
                  <span key={tag} className="rounded-full bg-bg-subtle px-2.5 py-1 text-sm text-text-muted">
                    {tag}
                  </span>
                ))}
              </div>
              <div className="mt-4 flex items-center gap-2 border-t border-border pt-3 text-sm text-text-muted">
                <Trophy size={15} strokeWidth={1.7} aria-hidden />
                <span className="truncate">{project.highestAward ?? '暂无获奖记录'}</span>
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

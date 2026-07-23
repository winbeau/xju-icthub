import { useQuery } from '@tanstack/react-query'
import { ArrowLeft, Trophy } from 'lucide-react'
import { Link, useParams } from 'react-router-dom'
import { getProject } from '@/api/endpoints/projects'
import { ProjectResourceWorkspace } from '@/components/resources/ProjectResourceWorkspace'
import { categoryColor } from '@/lib/projects'

export function ProjectDetailPage() {
  const { slug = '' } = useParams()
  const project = useQuery({
    queryKey: ['project', slug],
    queryFn: () => getProject(slug),
    enabled: Boolean(slug),
  })
  if (project.isLoading) return <PageMessage>项目加载中…</PageMessage>
  if (!project.data || project.isError) return <PageMessage error>项目不存在或加载失败。</PageMessage>

  const data = project.data
  return (
    <article className="mx-auto max-w-6xl px-5 py-10 sm:px-8 sm:py-12">
      <Link
        to="/projects"
        className="inline-flex items-center gap-2 text-sm text-text-muted transition-colors hover:text-text"
      >
        <ArrowLeft size={16} aria-hidden />
        返回项目库
      </Link>

      <header className="mt-7">
        <div className="flex flex-wrap items-center gap-2.5 text-sm">
          <span className="font-semibold" style={{ color: categoryColor(data.primaryCategory) }}>
            {data.primaryCategory}
          </span>
          {data.tags.map((tag) => (
            <span key={tag} className="rounded-full bg-bg-subtle px-2.5 py-0.5 text-text-muted">
              {tag}
            </span>
          ))}
        </div>
        <h1 className="mt-3 max-w-5xl font-serif text-4xl font-semibold leading-tight tracking-[-0.03em] sm:text-5xl">
          {data.name}
        </h1>
        <p className="mt-4 max-w-4xl text-lg leading-8 text-text-muted">{data.summary}</p>

        <div className="mt-5 flex flex-wrap gap-x-7 gap-y-2 text-sm text-text-muted">
          <span>
            来源者：<strong className="font-medium text-text">{data.sourceName ?? '待确认'}</strong>
          </span>
          <span>
            当前负责人：<strong className="font-medium text-text">{data.ownerName ?? '待确认'}</strong>
          </span>
          <span>
            当前状态：<strong className="font-medium text-text">{data.status}</strong>
          </span>
          {data.highestAward && (
            <span className="inline-flex items-center gap-1.5 text-text">
              <Trophy className="size-4 text-text-faint" strokeWidth={1.7} aria-hidden />
              {data.highestAward}
            </span>
          )}
        </div>
      </header>

      <div className="mt-8">
        <ProjectResourceWorkspace slug={data.slug} resources={data.resources} />
      </div>

      {data.critique && (
        <section className="mt-7 rounded-lg bg-bg-subtle px-5 py-4">
          <p className="text-xs font-semibold tracking-wide text-text-faint">项目备注</p>
          <p className="mt-1.5 leading-7 text-text-muted">{data.critique}</p>
        </section>
      )}
    </article>
  )
}

function PageMessage({ children, error = false }: { children: string; error?: boolean }) {
  return (
    <div
      className={`mx-auto max-w-6xl px-5 py-16 text-base sm:px-8 ${error ? 'text-cat-internet' : 'text-text-muted'}`}
    >
      {children}
    </div>
  )
}

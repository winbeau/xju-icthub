import { useQuery } from '@tanstack/react-query'
import {
  ArrowLeft,
  ArrowUpRight,
  FileArchive,
  FileImage,
  FileText,
  Github,
  Link as LinkIcon,
  Presentation,
  Trophy,
  Video,
} from 'lucide-react'
import { Link, useParams } from 'react-router-dom'
import { getProject } from '@/api/endpoints/projects'
import type { ProjectResource } from '@/api/schemas/project'
import { ProjectCover } from '@/components/projects/ProjectCover'
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
    <article className="mx-auto max-w-6xl px-5 py-12 sm:px-8 sm:py-16">
      <Link to="/projects" className="inline-flex items-center gap-2 text-base text-text-muted hover:text-text">
        <ArrowLeft size={17} aria-hidden />
        返回项目库
      </Link>

      <header className="mt-9 grid gap-8 border-b border-border pb-10 md:grid-cols-[minmax(0,1fr)_380px] md:items-center">
        <div>
          <span className="text-base font-semibold" style={{ color: categoryColor(data.primaryCategory) }}>
            {data.primaryCategory}
          </span>
          <h1 className="mt-3 max-w-4xl font-serif text-4xl font-semibold leading-tight tracking-[-0.03em] sm:text-5xl">
            {data.name}
          </h1>
          <p className="mt-5 max-w-3xl text-lg leading-8 text-text-muted">{data.summary}</p>
          <div className="mt-5 flex items-center gap-2 text-base text-text-muted">
            <Trophy size={17} aria-hidden />
            <span>曾获奖：{data.highestAward ?? '暂无'}</span>
          </div>
        </div>
        <ProjectCover cover={data} className="aspect-[16/10]" />
      </header>

      <div className="grid gap-12 py-10 md:grid-cols-[minmax(0,1fr)_280px]">
        <div>
          <section>
            <h2 className="font-serif text-2xl font-semibold">项目锐评</h2>
            <p className="mt-3 max-w-2xl leading-8 text-text-muted">{data.critique || '暂无锐评。'}</p>
          </section>
          <section className="mt-10">
            <h2 className="font-serif text-2xl font-semibold">相关资源</h2>
            <div className="mt-3 border-t border-border">
              {data.resources.map((resource) => <ResourceRow key={resource.id} resource={resource} />)}
              {data.resources.length === 0 && <p className="border-b border-border py-4 text-base text-text-muted">尚未整理资源。</p>}
            </div>
          </section>
        </div>
        <aside className="space-y-6 text-base">
          <Meta label="当前状态" value={data.status} />
          <Meta label="目前负责" value={data.ownerName ?? '待确认'} />
          <Meta label="来源者 / 方" value={data.sourceName ?? '待确认'} />
          <div>
            <h3 className="text-sm font-medium uppercase tracking-[0.12em] text-text-faint">项目标签</h3>
            <div className="mt-2 flex flex-wrap gap-2">
              {data.tags.map((tag) => <span key={tag} className="rounded-full bg-bg-subtle px-2.5 py-1 text-sm text-text-muted">{tag}</span>)}
            </div>
          </div>
        </aside>
      </div>
    </article>
  )
}

function PageMessage({ children, error = false }: { children: string; error?: boolean }) {
  return <div className={`mx-auto max-w-5xl px-5 py-16 text-base sm:px-8 ${error ? 'text-cat-internet' : 'text-text-muted'}`}>{children}</div>
}

function Meta({ label, value }: { label: string; value: string }) {
  return <div><h3 className="text-sm font-medium uppercase tracking-[0.12em] text-text-faint">{label}</h3><p className="mt-1.5 text-text">{value}</p></div>
}

function ResourceRow({ resource }: { resource: ProjectResource }) {
  const Icon = resourceIcon(resource.type)
  const content = <><Icon size={18} strokeWidth={1.7} aria-hidden /><span className="flex-1">{resource.title}</span>{resource.url && <ArrowUpRight size={16} aria-hidden />}</>
  if (resource.url) return <a href={resource.url} target="_blank" rel="noreferrer" className="flex items-center gap-3 border-b border-border py-4 text-base text-text-muted transition-colors hover:text-text">{content}</a>
  return <div className="flex items-center gap-3 border-b border-border py-4 text-base text-text-muted">{content}</div>
}

function resourceIcon(type: ProjectResource['type']) {
  switch (type) {
    case 'github': return Github
    case 'document': return FileText
    case 'presentation': return Presentation
    case 'archive': return FileArchive
    case 'video': return Video
    case 'image': return FileImage
    case 'baidu':
    case 'link': return LinkIcon
  }
}

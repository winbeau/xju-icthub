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
  const groups = groupResources(data.resources)
  return (
    <article className="mx-auto max-w-5xl px-5 py-12 sm:px-8 sm:py-16">
      <Link
        to="/projects"
        className="inline-flex items-center gap-2 text-sm text-text-muted transition-colors hover:text-text"
      >
        <ArrowLeft size={16} aria-hidden />
        返回项目库
      </Link>

      <header className="mt-8 border-b border-border pb-8">
        <div className="flex flex-wrap items-center gap-x-3 gap-y-2 text-sm">
          <span className="font-semibold" style={{ color: categoryColor(data.primaryCategory) }}>
            {data.primaryCategory}
          </span>
          <span className="text-text-faint">·</span>
          <span className="text-text-muted">{data.status}</span>
        </div>
        <h1 className="mt-4 max-w-4xl font-serif text-4xl font-semibold leading-tight tracking-[-0.03em] sm:text-5xl">
          {data.name}
        </h1>

        <div className="mt-6 flex flex-wrap items-center gap-x-5 gap-y-2.5 text-sm text-text-muted">
          <div className="flex flex-wrap gap-2">
            {data.tags.map((tag) => (
              <span key={tag} className="rounded-full bg-bg-subtle px-2.5 py-1">
                {tag}
              </span>
            ))}
          </div>
          <span>
            来源者：<span className="text-text">{data.sourceName ?? '待确认'}</span>
          </span>
          <span>
            当前负责人：<span className="text-text">{data.ownerName ?? '待确认'}</span>
          </span>
        </div>
      </header>

      <div>
        <ContentSection title="项目简介">
          <p className="max-w-3xl text-lg leading-8 text-text-muted">{data.summary}</p>
          {(data.highestAward || data.critique) && (
            <div className="mt-5 grid gap-3 text-sm sm:grid-cols-2 sm:gap-6">
              {data.highestAward && (
                <div className="flex gap-3 text-text-muted">
                  <Trophy className="mt-0.5 size-4 shrink-0" strokeWidth={1.7} aria-hidden />
                  <div>
                    <p className="text-text-faint">曾获奖</p>
                    <p className="mt-1 leading-6 text-text">{data.highestAward}</p>
                  </div>
                </div>
              )}
              {data.critique && (
                <div>
                  <p className="text-text-faint">项目备注</p>
                  <p className="mt-1 leading-6 text-text-muted">{data.critique}</p>
                </div>
              )}
            </div>
          )}
        </ContentSection>

        <ResourceSection title="项目链接" resources={groups.links} />
        <ResourceSection title="项目文档" resources={groups.documents} />
        <ResourceSection title="项目 PPT" resources={groups.presentations} />
        <ResourceSection title="展示视频" resources={groups.videos} />
        {groups.supplementary.length > 0 && (
          <ResourceSection title="补充材料" resources={groups.supplementary} />
        )}
      </div>
    </article>
  )
}

function ContentSection({ title, children }: { title: string; children: React.ReactNode }) {
  return (
    <section className="grid gap-3 py-7 sm:grid-cols-[132px_minmax(0,1fr)] sm:gap-6 sm:py-8">
      <h2 className="font-serif text-xl font-semibold">{title}</h2>
      <div>{children}</div>
    </section>
  )
}

function ResourceSection({
  title,
  resources,
}: {
  title: string
  resources: ProjectResource[]
}) {
  return (
    <section className="grid gap-1.5 py-2.5 sm:grid-cols-[132px_minmax(0,1fr)] sm:gap-6">
      <h2 className="pt-1.5 font-serif text-xl font-semibold">{title}</h2>
      <div className="space-y-1">
        {resources.map((resource) => (
          <ResourceRow key={resource.id} resource={resource} />
        ))}
        {resources.length === 0 && (
          <p className="px-3 py-2 text-sm text-text-faint">暂未收录</p>
        )}
      </div>
    </section>
  )
}

function PageMessage({ children, error = false }: { children: string; error?: boolean }) {
  return (
    <div
      className={`mx-auto max-w-5xl px-5 py-16 text-base sm:px-8 ${error ? 'text-cat-internet' : 'text-text-muted'}`}
    >
      {children}
    </div>
  )
}

function ResourceRow({ resource }: { resource: ProjectResource }) {
  const Icon = resourceIcon(resource.type)
  const provider = resourceProvider(resource)
  const content = (
    <>
      <Icon className="size-4 shrink-0 text-text-faint" strokeWidth={1.7} aria-hidden />
      <p className="min-w-0 flex-1 truncate text-base text-text">{resource.title}</p>
      <span className="hidden max-w-40 truncate text-xs text-text-faint sm:inline">{provider}</span>
      <span className="shrink-0 text-sm text-text-faint">
        {resource.url ? '打开' : '待补充'}
      </span>
      {resource.url && <ArrowUpRight className="size-4 shrink-0 text-text-faint" aria-hidden />}
    </>
  )
  const className =
    '-mx-3 flex items-center gap-3 rounded-md px-3 py-2 transition-colors hover:bg-bg-subtle hover:text-text'
  if (resource.url) {
    return (
      <a href={resource.url} target="_blank" rel="noreferrer" className={className}>
        {content}
      </a>
    )
  }
  return <div className={className}>{content}</div>
}

function groupResources(resources: ProjectResource[]) {
  return {
    links: resources.filter((resource) => ['github', 'baidu', 'link'].includes(resource.type)),
    documents: resources.filter((resource) => resource.type === 'document'),
    presentations: resources.filter((resource) => resource.type === 'presentation'),
    videos: resources.filter((resource) => resource.type === 'video'),
    supplementary: resources.filter((resource) => ['image', 'archive'].includes(resource.type)),
  }
}

function resourceProvider(resource: ProjectResource): string {
  if (!resource.url) return resourceTypeLabel(resource.type)
  try {
    return new URL(resource.url).hostname.replace(/^www\./, '')
  } catch {
    return resourceTypeLabel(resource.type)
  }
}

function resourceTypeLabel(type: ProjectResource['type']): string {
  switch (type) {
    case 'github':
      return 'GitHub'
    case 'baidu':
      return '百度网盘'
    case 'document':
      return '项目文档'
    case 'presentation':
      return '演示文稿'
    case 'archive':
      return '归档文件'
    case 'video':
      return '展示视频'
    case 'image':
      return '图片材料'
    case 'link':
      return '项目链接'
  }
}

function resourceIcon(type: ProjectResource['type']) {
  switch (type) {
    case 'github':
      return Github
    case 'document':
      return FileText
    case 'presentation':
      return Presentation
    case 'archive':
      return FileArchive
    case 'video':
      return Video
    case 'image':
      return FileImage
    case 'baidu':
    case 'link':
      return LinkIcon
  }
}

import { useEffect, useMemo, useState } from 'react'
import { Panel, PanelGroup, PanelResizeHandle } from 'react-resizable-panels'
import {
  ArrowUpRight,
  FileArchive,
  FileImage,
  FileText,
  Github,
  GripVertical,
  Link as LinkIcon,
  Presentation,
  Video,
} from 'lucide-react'
import type { ProjectResource } from '@/api/schemas/project'
import { ResourcePreviewPane } from '@/components/resources/ResourcePreviewPane'

type Props = {
  slug: string
  resources: ProjectResource[]
}

export function ProjectResourceWorkspace({ slug, resources }: Props) {
  const previewable = useMemo(
    () => resources.filter((resource) => resource.contentUrl),
    [resources],
  )
  const [selectedId, setSelectedId] = useState<string | null>(previewable[0]?.id ?? null)

  useEffect(() => {
    if (selectedId && resources.some((resource) => resource.id === selectedId)) return
    setSelectedId(previewable[0]?.id ?? null)
  }, [previewable, resources, selectedId])

  const selected = resources.find((resource) => resource.id === selectedId) ?? null
  const groups = groupResources(resources)

  return (
    <section className="overflow-hidden rounded-xl border border-border bg-bg shadow-sm">
      <div className="md:hidden">
        <div className="max-h-72 overflow-y-auto">
          <ResourceNavigation
            groups={groups}
            resourceCount={resources.length}
            selectedId={selectedId}
            onSelect={setSelectedId}
          />
        </div>
        <div className="h-[64vh] min-h-[440px] border-t border-border">
          <ResourcePreviewPane slug={slug} resource={selected} />
        </div>
      </div>
      <PanelGroup direction="horizontal" className="hidden h-[68vh] min-h-[560px] md:flex">
        <Panel defaultSize={31} minSize={24} maxSize={42} className="min-w-[250px]">
          <ResourceNavigation
            groups={groups}
            resourceCount={resources.length}
            selectedId={selectedId}
            onSelect={setSelectedId}
          />
        </Panel>
        <PanelResizeHandle className="group relative w-px bg-border transition-colors hover:bg-border-strong">
          <span className="absolute left-1/2 top-1/2 flex h-8 w-4 -translate-x-1/2 -translate-y-1/2 items-center justify-center rounded-full border border-border bg-bg text-text-faint opacity-0 shadow-sm transition-opacity group-hover:opacity-100">
            <GripVertical className="size-3" aria-hidden />
          </span>
        </PanelResizeHandle>
        <Panel minSize={48}>
          <ResourcePreviewPane slug={slug} resource={selected} />
        </Panel>
      </PanelGroup>
    </section>
  )
}

function ResourceNavigation({
  groups,
  resourceCount,
  selectedId,
  onSelect,
}: {
  groups: ReturnType<typeof groupResources>
  resourceCount: number
  selectedId: string | null
  onSelect: (id: string) => void
}) {
  return (
    <div className="flex h-full min-h-0 flex-col">
      <div className="border-b border-border px-5 py-4">
        <h2 className="font-serif text-xl font-semibold">项目资料</h2>
        <p className="mt-1 text-sm text-text-muted">
          {resourceCount ? `${resourceCount} 项资料` : '暂未收录资料'}
        </p>
      </div>
      <div className="min-h-0 flex-1 overflow-y-auto px-2 py-3">
        {Object.entries(groups).map(([label, items]) =>
          items.length ? (
            <div key={label} className="mb-4 last:mb-0">
              <p className="px-3 pb-1.5 text-xs font-semibold tracking-wide text-text-faint">
                {label}
              </p>
              <div className="space-y-0.5">
                {items.map((resource) => (
                  <ResourceListItem
                    key={resource.id}
                    resource={resource}
                    active={resource.id === selectedId}
                    onSelect={() => onSelect(resource.id)}
                  />
                ))}
              </div>
            </div>
          ) : null,
        )}
        {!resourceCount && (
          <p className="px-3 py-8 text-center text-sm text-text-faint">暂无可查看资料</p>
        )}
      </div>
    </div>
  )
}

function ResourceListItem({
  resource,
  active,
  onSelect,
}: {
  resource: ProjectResource
  active: boolean
  onSelect: () => void
}) {
  const Icon = resourceIcon(resource.type)
  const internal = Boolean(resource.contentUrl)
  const className = `flex w-full items-center gap-3 rounded-lg px-3 py-2.5 text-left transition-colors ${
    active ? 'bg-bg-subtle text-text' : 'text-text-muted hover:bg-bg-subtle/70 hover:text-text'
  }`
  const body = (
    <>
      <Icon className="size-4 shrink-0 text-text-faint" strokeWidth={1.7} aria-hidden />
      <span className="min-w-0 flex-1 truncate text-sm font-medium" title={resource.title}>
        {resource.title}
      </span>
      {resource.previewKind && (
        <span className="shrink-0 text-[11px] uppercase text-text-faint">
          {previewLabel(resource.previewKind)}
        </span>
      )}
      {!internal && resource.url && <ArrowUpRight className="size-3.5 shrink-0" aria-hidden />}
    </>
  )
  if (!internal && resource.url) {
    return (
      <a className={className} href={resource.url} target="_blank" rel="noreferrer">
        {body}
      </a>
    )
  }
  return (
    <button type="button" className={className} onClick={onSelect}>
      {body}
    </button>
  )
}

function groupResources(resources: ProjectResource[]) {
  return {
    项目链接: resources.filter((resource) => ['github', 'baidu', 'link'].includes(resource.type)),
    项目文档: resources.filter((resource) => resource.type === 'document'),
    '项目 PPT': resources.filter((resource) => resource.type === 'presentation'),
    展示视频: resources.filter((resource) => resource.type === 'video'),
    补充材料: resources.filter((resource) => ['image', 'archive'].includes(resource.type)),
  }
}

function previewLabel(kind: NonNullable<ProjectResource['previewKind']>): string {
  if (kind === 'html_bundle') return 'HTML'
  if (kind === 'download') return '文件'
  return kind
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

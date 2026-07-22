import { useEffect, useMemo, useState } from 'react'
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import {
  ArrowLeft,
  FileText,
  Film,
  Github,
  LoaderCircle,
  Presentation,
  Search,
  Upload,
  X,
} from 'lucide-react'
import { Link, useLocation, useNavigate } from 'react-router-dom'
import { toast } from 'sonner'
import { createProject } from '@/api/endpoints/projects'
import { listTags } from '@/api/endpoints/tags'
import {
  ProjectWriteInputSchema,
  type ProjectResourceInput,
  type ProjectWriteInput,
} from '@/api/schemas/project'
import { CreateModeSwitch } from '@/components/projects/CreateModeSwitch'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { Label } from '@/components/ui/label'
import { Textarea } from '@/components/ui/textarea'
import { buildCoverPreview } from '@/lib/covers'
import { PROJECT_CATEGORIES } from '@/lib/projects'

type CoreResourceKey = 'github' | 'document' | 'presentation' | 'video'
type CoreResourceValues = Record<CoreResourceKey, string>

const CORE_RESOURCES: {
  key: CoreResourceKey
  type: ProjectResourceInput['type']
  title: string
  description: string
  placeholder: string
  icon: typeof Github
}[] = [
  {
    key: 'github',
    type: 'github',
    title: '项目源码',
    description: 'GitHub 仓库',
    placeholder: 'https://github.com/organization/repository',
    icon: Github,
  },
  {
    key: 'document',
    type: 'document',
    title: '项目文档',
    description: '说明书、论文或网盘链接',
    placeholder: '粘贴文档或网盘链接',
    icon: FileText,
  },
  {
    key: 'presentation',
    type: 'presentation',
    title: '项目 PPT',
    description: '答辩或展示材料',
    placeholder: '粘贴 PPT 或网盘链接',
    icon: Presentation,
  },
  {
    key: 'video',
    type: 'video',
    title: '展示视频',
    description: '演示、路演或介绍视频',
    placeholder: '粘贴视频或网盘链接',
    icon: Film,
  },
]

const EMPTY_CORE_RESOURCES: CoreResourceValues = {
  github: '',
  document: '',
  presentation: '',
  video: '',
}

function initialSlug(): string {
  return `project-${Date.now().toString(36)}`
}

export function ProjectCreatePage() {
  const navigate = useNavigate()
  const location = useLocation()
  const queryClient = useQueryClient()
  const imported = (location.state as { importDraft?: Partial<ProjectWriteInput> } | null)
    ?.importDraft
  const [name, setName] = useState(imported?.name ?? '')
  const [coreResources, setCoreResources] = useState<CoreResourceValues>(() =>
    resourcesToCore(imported?.resources ?? []),
  )
  const [slug, setSlug] = useState(imported?.slug ?? initialSlug)
  const [summary, setSummary] = useState(imported?.summary ?? '')
  const [primaryCategory, setPrimaryCategory] = useState<ProjectWriteInput['primaryCategory']>(
    imported?.primaryCategory ?? '传统软件',
  )
  const [sourceName, setSourceName] = useState(imported?.sourceName ?? '')
  const [ownerName, setOwnerName] = useState(imported?.ownerName ?? '')
  const [selectedTags, setSelectedTags] = useState<string[]>(imported?.tags ?? [])
  const [tagSearch, setTagSearch] = useState('')
  const [formError, setFormError] = useState('')
  const importedMaterials = useMemo(
    () =>
      (imported?.resources ?? []).filter(
        (resource) => !CORE_RESOURCES.some((item) => item.type === resource.type && resource.url),
      ),
    [imported?.resources],
  )
  const tags = useQuery({ queryKey: ['tags'], queryFn: () => listTags() })
  const visibleTags = (tags.data ?? []).filter((tag) =>
    `${tag.name} ${tag.groupName}`
      .toLocaleLowerCase('zh-CN')
      .includes(tagSearch.trim().toLocaleLowerCase('zh-CN')),
  )

  useEffect(() => {
    if (!imported) return
    window.history.replaceState({}, document.title)
  }, [imported])

  const save = useMutation({
    mutationFn: (input: ProjectWriteInput) => createProject(input),
    onSuccess: async (saved) => {
      await queryClient.invalidateQueries({ queryKey: ['projects'] })
      queryClient.setQueryData(['project', saved.slug], saved)
      toast.success('项目已上传')
      navigate(`/projects/${saved.slug}`)
    },
    onError: (error) =>
      setFormError(error instanceof Error ? error.message : '上传失败，请稍后重试'),
  })

  const submit = (event: React.FormEvent) => {
    event.preventDefault()
    const resources = [
      ...CORE_RESOURCES.flatMap<ProjectResourceInput>((resource) => {
        const url = coreResources[resource.key].trim()
        return url ? [{ type: resource.type, title: resource.title, url }] : []
      }),
      ...importedMaterials,
    ]
    const cover = buildCoverPreview({
      name,
      summary,
      primaryCategory,
      tags: selectedTags,
      resources,
    })
    const parsed = ProjectWriteInputSchema.safeParse({
      slug,
      name,
      summary,
      primaryCategory,
      highestAward: imported?.highestAward ?? null,
      status: imported?.status ?? '研发中',
      critique: imported?.critique ?? '',
      ownerName: ownerName.trim() || null,
      sourceName: sourceName.trim() || null,
      tags: selectedTags,
      resources,
      coverMode: cover.coverMode,
      coverTitle: cover.coverTitle,
      coverSubtitle: cover.coverSubtitle,
      coverKeywords: cover.coverKeywords,
      coverTone: cover.coverTone,
    })
    if (!parsed.success) {
      setFormError(parsed.error.issues[0]?.message ?? '请检查填写内容')
      return
    }
    setFormError('')
    save.mutate(parsed.data)
  }

  const toggleTag = (tag: string) => {
    setSelectedTags((current) =>
      current.includes(tag) ? current.filter((item) => item !== tag) : [...current, tag],
    )
  }

  return (
    <div className="mx-auto max-w-5xl px-5 py-12 sm:px-8 sm:py-16">
      <Button asChild variant="ghost" size="sm" className="-ml-3">
        <Link to="/admin/projects">
          <ArrowLeft aria-hidden />返回项目管理
        </Link>
      </Button>

      <header className="mt-6 flex flex-col gap-6 border-b border-border pb-7 sm:flex-row sm:items-end sm:justify-between">
        <div>
          <p className="text-sm font-medium uppercase tracking-[0.14em] text-text-faint">
            Upload Project
          </p>
          <h1 className="mt-3 font-serif text-4xl font-semibold tracking-[-0.025em]">上传项目</h1>
        </div>
        <CreateModeSwitch mode="manual" />
      </header>

      <form onSubmit={submit} className="pt-8">
        <section>
          <div className="max-w-3xl">
            <Label htmlFor="project-name" className="text-base font-medium">
              项目名
            </Label>
            <Input
              autoFocus
              id="project-name"
              value={name}
              onChange={(event) => setName(event.target.value)}
              placeholder="这个项目叫什么"
              className="mt-3 h-14 px-4 text-lg font-medium"
            />
          </div>

          <div className="mt-7 grid gap-4 sm:grid-cols-2">
            {CORE_RESOURCES.map((resource) => {
              const Icon = resource.icon
              return (
                <div key={resource.key} className="rounded-xl border border-border p-4">
                  <div className="flex items-center gap-3">
                    <Icon className="size-5 text-text-muted" aria-hidden />
                    <div>
                      <Label htmlFor={`resource-${resource.key}`} className="text-base font-medium">
                        {resource.title}
                      </Label>
                      <p className="mt-0.5 text-sm text-text-faint">{resource.description}</p>
                    </div>
                  </div>
                  <Input
                    id={`resource-${resource.key}`}
                    type="url"
                    value={coreResources[resource.key]}
                    onChange={(event) =>
                      setCoreResources((current) => ({
                        ...current,
                        [resource.key]: event.target.value,
                      }))
                    }
                    placeholder={resource.placeholder}
                    className="mt-4 h-11"
                  />
                </div>
              )
            })}
          </div>
        </section>

        <section className="mt-10 border-t border-border pt-8">
          <div>
            <h2 className="text-base font-medium">补充信息</h2>
            <p className="mt-1 text-sm text-text-muted">用于介绍、分类和交接，不抢占项目主体。</p>
          </div>

          <div className="mt-6 grid gap-5 sm:grid-cols-2">
            <Field label="项目简介" htmlFor="project-summary" className="sm:col-span-2">
              <Textarea
                id="project-summary"
                value={summary}
                onChange={(event) => setSummary(event.target.value)}
                placeholder="用一两句话说明项目解决什么问题"
                className="min-h-20 text-base leading-7"
              />
            </Field>
            <Field label="项目类别" htmlFor="project-category">
              <select
                id="project-category"
                value={primaryCategory}
                onChange={(event) =>
                  setPrimaryCategory(event.target.value as ProjectWriteInput['primaryCategory'])
                }
                className="flex h-11 w-full rounded-md border border-input bg-bg px-3 text-base text-text shadow-sm focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring"
              >
                {PROJECT_CATEGORIES.map((category) => (
                  <option key={category} value={category}>
                    {category}
                  </option>
                ))}
              </select>
            </Field>
            <Field label="项目路径" htmlFor="project-slug" hint="用于详情页 URL">
              <Input
                id="project-slug"
                value={slug}
                onChange={(event) => setSlug(event.target.value.toLowerCase())}
                placeholder="project-slug"
                className="h-11"
              />
            </Field>
            <Field label="来源者 / 方" htmlFor="project-source">
              <Input
                id="project-source"
                value={sourceName}
                onChange={(event) => setSourceName(event.target.value)}
                placeholder="个人、团队或项目来源"
                className="h-11"
              />
            </Field>
            <Field label="当前负责" htmlFor="project-owner">
              <Input
                id="project-owner"
                value={ownerName}
                onChange={(event) => setOwnerName(event.target.value)}
                placeholder="当前负责人"
                className="h-11"
              />
            </Field>

            <div className="sm:col-span-2">
              <div className="flex items-center justify-between gap-4">
                <Label htmlFor="tag-search">标签</Label>
                <span className="text-xs text-text-faint">已选 {selectedTags.length}</span>
              </div>
              <div className="relative mt-2 max-w-sm">
                <Search
                  className="pointer-events-none absolute left-3 top-1/2 size-4 -translate-y-1/2 text-text-faint"
                  aria-hidden
                />
                <Input
                  id="tag-search"
                  value={tagSearch}
                  onChange={(event) => setTagSearch(event.target.value)}
                  placeholder="搜索已有标签"
                  className="h-10 pl-9"
                />
              </div>
              {selectedTags.length > 0 && (
                <div className="mt-3 flex flex-wrap gap-2">
                  {selectedTags.map((tag) => (
                    <button
                      type="button"
                      key={tag}
                      onClick={() => toggleTag(tag)}
                      className="inline-flex items-center gap-1.5 rounded-full bg-bg-subtle px-3 py-1.5 text-sm text-text"
                    >
                      {tag}<X className="size-3.5 text-text-faint" aria-hidden />
                    </button>
                  ))}
                </div>
              )}
              <div className="mt-3 flex max-h-32 flex-wrap gap-2 overflow-y-auto">
                {visibleTags
                  .filter((tag) => !selectedTags.includes(tag.name))
                  .map((tag) => (
                    <button
                      type="button"
                      key={tag.id}
                      onClick={() => toggleTag(tag.name)}
                      className="rounded-full border border-border px-3 py-1.5 text-sm text-text-muted transition-colors hover:border-border-strong hover:text-text"
                    >
                      {tag.name}
                    </button>
                  ))}
              </div>
            </div>
          </div>

          {importedMaterials.length > 0 && (
            <div className="mt-6 rounded-lg bg-bg-subtle p-4">
              <p className="text-sm font-medium">Codex 已整理的附件</p>
              <ul className="mt-2 space-y-1 text-sm text-text-muted">
                {importedMaterials.slice(0, 8).map((resource, index) => (
                  <li key={`${resource.type}-${resource.title}-${index}`}>{resource.title}</li>
                ))}
              </ul>
            </div>
          )}
        </section>

        <div className="mt-8 flex flex-col-reverse gap-3 border-t border-border pt-6 sm:flex-row sm:items-center sm:justify-between">
          <p className="text-sm text-red-600">{formError}</p>
          <Button size="lg" type="submit" disabled={save.isPending}>
            {save.isPending ? <LoaderCircle className="animate-spin" aria-hidden /> : <Upload aria-hidden />}
            {save.isPending ? '正在上传…' : '上传项目'}
          </Button>
        </div>
      </form>
    </div>
  )
}

function Field({
  children,
  className,
  hint,
  htmlFor,
  label,
}: {
  children: React.ReactNode
  className?: string
  hint?: string
  htmlFor: string
  label: string
}) {
  return (
    <div className={className}>
      <div className="flex items-center justify-between gap-3">
        <Label htmlFor={htmlFor}>{label}</Label>
        {hint && <span className="text-xs text-text-faint">{hint}</span>}
      </div>
      <div className="mt-2">{children}</div>
    </div>
  )
}

function resourcesToCore(resources: ProjectResourceInput[]): CoreResourceValues {
  const values = { ...EMPTY_CORE_RESOURCES }
  for (const resource of resources) {
    if (!resource.url) continue
    const key = CORE_RESOURCES.find((item) => item.type === resource.type)?.key
    if (key && !values[key]) values[key] = resource.url
  }
  return values
}

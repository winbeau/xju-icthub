import { useEffect, useMemo, useState } from 'react'
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { ArrowLeft, Plus, RefreshCw, Search, Sparkles, Trash2, X } from 'lucide-react'
import { Link, useLocation, useNavigate, useParams } from 'react-router-dom'
import { toast } from 'sonner'
import {
  createProject,
  generateProjectCover,
  getProject,
  updateProject,
} from '@/api/endpoints/projects'
import { createTag, listTags, suggestTag } from '@/api/endpoints/tags'
import {
  ProjectWriteInputSchema,
  type ProjectCover,
  type ProjectResourceInput,
  type ProjectWriteInput,
} from '@/api/schemas/project'
import { canManageTags } from '@/api/schemas/user'
import type { TagDefinition } from '@/api/schemas/tag'
import { ProjectCover as ProjectCoverPreview } from '@/components/projects/ProjectCover'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { Label } from '@/components/ui/label'
import { Textarea } from '@/components/ui/textarea'
import { buildCoverPreview, isImageUrl } from '@/lib/covers'
import { PROJECT_CATEGORIES } from '@/lib/projects'
import { useAuthStore } from '@/stores/authStore'

const PROJECT_STATUSES = ['研发中', '运维测试', '迁移中', '已上线', '暂停维护', '已归档']
const RESOURCE_TYPES: { value: ProjectResourceInput['type']; label: string }[] = [
  { value: 'github', label: 'GitHub' },
  { value: 'baidu', label: '百度网盘' },
  { value: 'image', label: '截图 / 海报' },
  { value: 'presentation', label: 'PPT' },
  { value: 'document', label: '文档 / PDF' },
  { value: 'archive', label: '压缩包' },
  { value: 'video', label: '展示视频' },
  { value: 'link', label: '其他链接' },
]

const EMPTY_PROJECT: ProjectWriteInput = {
  slug: '',
  name: '',
  summary: '',
  primaryCategory: '传统软件',
  highestAward: null,
  status: '研发中',
  critique: '',
  ownerName: null,
  sourceName: null,
  tags: [],
  resources: [],
  coverMode: 'text',
  coverTitle: null,
  coverSubtitle: null,
  coverKeywords: [],
  coverTone: 'slate',
}

export function ProjectEditorPage() {
  const { slug } = useParams()
  const editing = Boolean(slug)
  const navigate = useNavigate()
  const location = useLocation()
  const queryClient = useQueryClient()
  const user = useAuthStore((state) => state.user)
  const tagAdmin = canManageTags(user)
  const [form, setForm] = useState<ProjectWriteInput>(EMPTY_PROJECT)
  const [manualCover, setManualCover] = useState(false)
  const [tagSearch, setTagSearch] = useState('')
  const [newTagName, setNewTagName] = useState('')
  const [newTagGroup, setNewTagGroup] = useState('技术')
  const [formError, setFormError] = useState('')

  useEffect(() => {
    if (editing) return
    const state = location.state as { importDraft?: Partial<ProjectWriteInput> } | null
    if (!state?.importDraft) return
    setForm((current) => ({ ...current, ...state.importDraft }))
    window.history.replaceState({}, document.title)
  }, [editing, location.state])

  const project = useQuery({
    queryKey: ['project', slug],
    queryFn: () => getProject(slug!),
    enabled: editing,
  })
  const tags = useQuery({ queryKey: ['tags'], queryFn: () => listTags() })

  useEffect(() => {
    if (!project.data) return
    setForm({
      slug: project.data.slug,
      name: project.data.name,
      summary: project.data.summary,
      primaryCategory: project.data.primaryCategory,
      highestAward: project.data.highestAward,
      status: project.data.status,
      critique: project.data.critique,
      ownerName: project.data.ownerName,
      sourceName: project.data.sourceName,
      tags: project.data.tags,
      resources: project.data.resources.map(({ id: _id, ...resource }) => resource),
      coverMode: project.data.coverMode,
      coverTitle: project.data.coverTitle,
      coverSubtitle: project.data.coverSubtitle,
      coverKeywords: project.data.coverKeywords,
      coverTone: project.data.coverTone,
    })
    setManualCover(project.data.coverMode === 'manual')
  }, [project.data])

  const automaticCover = useMemo(
    () =>
      buildCoverPreview({
        name: form.name,
        summary: form.summary,
        primaryCategory: form.primaryCategory,
        tags: form.tags,
        resources: form.resources,
      }),
    [form.name, form.primaryCategory, form.resources, form.summary, form.tags],
  )
  const cover: ProjectCover = manualCover
    ? {
        ...automaticCover,
        coverMode: 'manual',
        coverTitle: form.coverTitle || automaticCover.coverTitle,
        coverSubtitle: form.coverSubtitle || automaticCover.coverSubtitle,
        coverKeywords: form.coverKeywords.length ? form.coverKeywords : automaticCover.coverKeywords,
        coverTone: form.coverTone,
        coverConfidence: 1,
      }
    : automaticCover

  const save = useMutation({
    mutationFn: (input: ProjectWriteInput) =>
      editing ? updateProject(slug!, input) : createProject(input),
    onSuccess: async (saved) => {
      await queryClient.invalidateQueries({ queryKey: ['projects'] })
      queryClient.setQueryData(['project', saved.slug], saved)
      toast.success(editing ? '项目已更新' : '项目已上传')
      navigate('/admin/projects')
    },
    onError: (error) => setFormError(error instanceof Error ? error.message : '保存失败'),
  })
  const generateCover = useMutation({
    mutationFn: () => generateProjectCover(slug!),
    onSuccess: (generated) => {
      setManualCover(false)
      setForm((current) => ({
        ...current,
        coverMode: generated.coverMode,
        coverTitle: generated.coverTitle,
        coverSubtitle: generated.coverSubtitle,
        coverKeywords: generated.coverKeywords,
        coverTone: generated.coverTone,
      }))
      queryClient.setQueryData(['project', slug], (current: unknown) =>
        typeof current === 'object' && current ? { ...current, ...generated } : current,
      )
      toast.success('封面已重新生成')
    },
    onError: (error) => toast.error(error instanceof Error ? error.message : '封面生成失败'),
  })
  const addTag = useMutation({
    mutationFn: () =>
      createTag({ name: newTagName, groupName: newTagGroup, color: null, sortOrder: 700 }),
    onSuccess: async (created) => {
      await queryClient.invalidateQueries({ queryKey: ['tags'] })
      setField('tags', [...form.tags, created.name])
      setNewTagName('')
      toast.success('正式标签已添加')
    },
    onError: (error) => toast.error(error instanceof Error ? error.message : '标签添加失败'),
  })
  const suggest = useMutation({
    mutationFn: () => suggestTag({ name: newTagName, groupName: newTagGroup, reason: null }),
    onSuccess: () => {
      setNewTagName('')
      toast.success('标签建议已提交，等待管理员审核')
    },
    onError: (error) => toast.error(error instanceof Error ? error.message : '建议提交失败'),
  })

  const submit = (event: React.FormEvent) => {
    event.preventDefault()
    const parsed = ProjectWriteInputSchema.safeParse({
      ...form,
      coverMode: cover.coverMode,
      coverTitle: cover.coverTitle,
      coverSubtitle: cover.coverSubtitle,
      coverKeywords: cover.coverKeywords,
      coverTone: cover.coverTone,
    })
    if (!parsed.success) {
      setFormError(parsed.error.issues[0]?.message ?? '请检查表单')
      return
    }
    setFormError('')
    save.mutate(parsed.data)
  }

  const setField = <K extends keyof ProjectWriteInput>(key: K, value: ProjectWriteInput[K]) => {
    setForm((current) => ({ ...current, [key]: value }))
  }
  const setResource = (index: number, resource: ProjectResourceInput) => {
    setForm((current) => ({
      ...current,
      resources: current.resources.map((item, itemIndex) => (itemIndex === index ? resource : item)),
    }))
  }
  const toggleTag = (tag: string) => {
    setField('tags', form.tags.includes(tag) ? form.tags.filter((item) => item !== tag) : [...form.tags, tag])
  }
  const visibleTags = (tags.data ?? []).filter((tag) =>
    `${tag.name} ${tag.groupName}`.toLocaleLowerCase('zh-CN').includes(tagSearch.trim().toLocaleLowerCase('zh-CN')),
  )

  if (editing && project.isLoading) return <PageMessage>正在加载项目…</PageMessage>
  if (editing && project.isError) return <PageMessage error>项目加载失败。</PageMessage>

  return (
    <div className="mx-auto max-w-5xl px-5 py-12 sm:px-8 sm:py-16">
      <Button asChild variant="ghost" size="sm" className="-ml-3">
        <Link to="/admin/projects"><ArrowLeft aria-hidden />返回项目管理</Link>
      </Button>
      <div className="mt-6 border-b border-border pb-7">
        <p className="text-sm font-medium uppercase tracking-[0.14em] text-text-faint">{editing ? 'Edit Project' : 'Upload Project'}</p>
        <h1 className="mt-3 font-serif text-4xl font-semibold">{editing ? '编辑项目' : '上传项目'}</h1>
      </div>

      <form onSubmit={submit} className="space-y-12 pt-8">
        <section className="grid gap-6 sm:grid-cols-2">
          <Field label="项目名" htmlFor="project-name" className="sm:col-span-2">
            <Input id="project-name" value={form.name} onChange={(event) => setField('name', event.target.value)} className="h-11" />
          </Field>
          <Field label="项目路径" htmlFor="project-slug" hint="用于 URL，例如 lab-device-booking">
            <Input id="project-slug" value={form.slug} onChange={(event) => setField('slug', event.target.value.toLowerCase())} placeholder="lowercase-slug" className="h-11" />
          </Field>
          <Field label="主分类" htmlFor="project-category" hint="必须且只能选择一项">
            <select id="project-category" value={form.primaryCategory} onChange={(event) => setField('primaryCategory', event.target.value as ProjectWriteInput['primaryCategory'])} className="h-11 w-full rounded-md border border-input bg-bg px-3 text-base shadow-sm focus:outline-none focus:ring-1 focus:ring-ring">
              {PROJECT_CATEGORIES.map((category) => <option key={category}>{category}</option>)}
            </select>
          </Field>
          <Field label="一句话简介" htmlFor="project-summary" className="sm:col-span-2">
            <Textarea id="project-summary" value={form.summary} onChange={(event) => setField('summary', event.target.value)} className="min-h-28 text-base" />
          </Field>
          <Field label="曾获奖" htmlFor="project-award"><Input id="project-award" value={form.highestAward ?? ''} onChange={(event) => setField('highestAward', event.target.value || null)} placeholder="没有可留空" className="h-11" /></Field>
          <Field label="目前状态" htmlFor="project-status"><select id="project-status" value={form.status} onChange={(event) => setField('status', event.target.value)} className="h-11 w-full rounded-md border border-input bg-bg px-3 text-base shadow-sm">{PROJECT_STATUSES.map((status) => <option key={status}>{status}</option>)}</select></Field>
          <Field label="目前负责" htmlFor="project-owner"><Input id="project-owner" value={form.ownerName ?? ''} onChange={(event) => setField('ownerName', event.target.value || null)} className="h-11" /></Field>
          <Field label="来源者 / 方" htmlFor="project-source"><Input id="project-source" value={form.sourceName ?? ''} onChange={(event) => setField('sourceName', event.target.value || null)} className="h-11" /></Field>
        </section>

        <section>
          <SectionTitle title="项目标签" description="选择已有标签；比赛名称在这里标记，不再作为主分类。" />
          <div className="mt-4 flex flex-wrap gap-2">
            {form.tags.map((tag) => <button key={tag} type="button" onClick={() => toggleTag(tag)} className="inline-flex items-center gap-1 rounded-full bg-text px-3 py-1.5 text-sm text-white">{tag}<X size={14} aria-hidden /></button>)}
          </div>
          <div className="relative mt-4 max-w-md"><Search size={17} className="pointer-events-none absolute left-3 top-1/2 -translate-y-1/2 text-text-faint" aria-hidden /><Input value={tagSearch} onChange={(event) => setTagSearch(event.target.value)} placeholder="搜索标签或分组" className="h-11 pl-10" /></div>
          <div className="mt-4 max-h-64 space-y-4 overflow-y-auto rounded-lg border border-border p-4">
            {Object.entries(groupTags(visibleTags)).map(([group, groupTags]) => (
              <div key={group}><p className="mb-2 text-sm font-semibold text-text-faint">{group}</p><div className="flex flex-wrap gap-2">{groupTags?.map((tag) => <button key={tag.id} type="button" onClick={() => toggleTag(tag.name)} className={`rounded-full border px-3 py-1.5 text-sm ${form.tags.includes(tag.name) ? 'border-text bg-text text-white' : 'border-border text-text-muted hover:border-border-strong hover:text-text'}`}>{tag.name}</button>)}</div></div>
            ))}
            {!visibleTags.length && <p className="text-base text-text-muted">没有匹配的正式标签。</p>}
          </div>
          <div className="mt-4 grid gap-3 rounded-lg bg-bg-subtle p-4 sm:grid-cols-[1fr_160px_auto] sm:items-end">
            <Field label={tagAdmin ? '快速新增正式标签' : '提交标签建议'} htmlFor="new-tag"><Input id="new-tag" value={newTagName} onChange={(event) => setNewTagName(event.target.value)} placeholder="标签名称" className="h-11" /></Field>
            <Field label="分组" htmlFor="new-tag-group"><select id="new-tag-group" value={newTagGroup} onChange={(event) => setNewTagGroup(event.target.value)} className="h-11 w-full rounded-md border border-input bg-bg px-3 text-base"><option>比赛</option><option>技术</option><option>特征</option><option>领域</option><option>来源</option></select></Field>
            <Button type="button" variant="outline" disabled={!newTagName.trim() || addTag.isPending || suggest.isPending} onClick={() => tagAdmin ? addTag.mutate() : suggest.mutate()}>{tagAdmin ? '新增标签' : '提交建议'}</Button>
          </div>
        </section>

        <section>
          <SectionTitle title="项目资源" description="支持图片、PPT、视频、GitHub、文档、网盘和压缩包。图片链接会自动识别为封面候选。" action={<Button type="button" variant="outline" size="sm" onClick={() => setField('resources', [...form.resources, { type: 'github', title: '', url: null }])}><Plus aria-hidden />添加资源</Button>} />
          <div className="space-y-3 pt-4">
            {form.resources.map((resource, index) => (
              <div key={index} className="grid gap-3 rounded-lg border border-border p-3 sm:grid-cols-[150px_1fr_1.3fr_auto]">
                <select value={resource.type} onChange={(event) => setResource(index, { ...resource, type: event.target.value as ProjectResourceInput['type'] })} className="h-11 rounded-md border border-input bg-bg px-3 text-base" aria-label="资源类型">{RESOURCE_TYPES.map((type) => <option key={type.value} value={type.value}>{type.label}</option>)}</select>
                <Input value={resource.title} onChange={(event) => setResource(index, { ...resource, title: event.target.value })} placeholder="资源标题" aria-label="资源标题" className="h-11" />
                <Input value={resource.url ?? ''} onChange={(event) => { const url = event.target.value || null; setResource(index, { ...resource, url, type: isImageUrl(url) ? 'image' : resource.type }) }} placeholder="https://…（可稍后补）" aria-label="资源链接" className="h-11" />
                <Button type="button" variant="ghost" size="icon" aria-label="移除资源" onClick={() => setField('resources', form.resources.filter((_, itemIndex) => itemIndex !== index))}><Trash2 aria-hidden /></Button>
              </div>
            ))}
            {!form.resources.length && <p className="py-4 text-base text-text-muted">暂不添加资源也可以，系统会先使用文字封面。</p>}
          </div>
        </section>

        <section>
          <SectionTitle title="Agent 封面" description="优先使用截图或海报；没有图片时生成稳定的文字封面。" action={<Button type="button" variant="outline" size="sm" disabled={generateCover.isPending} onClick={() => editing ? generateCover.mutate() : setManualCover(false)}>{generateCover.isPending ? <RefreshCw className="animate-spin" aria-hidden /> : <Sparkles aria-hidden />}重新生成</Button>} />
          <div className="mt-5 grid gap-6 md:grid-cols-[340px_minmax(0,1fr)]">
            <ProjectCoverPreview cover={cover} className="aspect-[16/10]" />
            <div className="space-y-4">
              <label className="flex items-center gap-2 text-base"><input type="checkbox" checked={manualCover} onChange={(event) => { setManualCover(event.target.checked); if (event.target.checked) setForm((current) => ({ ...current, coverTitle: cover.coverTitle, coverSubtitle: cover.coverSubtitle, coverKeywords: cover.coverKeywords, coverTone: cover.coverTone })) }} />手动覆盖 Agent 文字</label>
              {manualCover && <div className="grid gap-4 sm:grid-cols-2"><Field label="封面标题" htmlFor="cover-title"><Input id="cover-title" value={form.coverTitle ?? ''} onChange={(event) => setField('coverTitle', event.target.value)} maxLength={16} className="h-11" /></Field><Field label="副标题" htmlFor="cover-subtitle"><Input id="cover-subtitle" value={form.coverSubtitle ?? ''} onChange={(event) => setField('coverSubtitle', event.target.value)} maxLength={40} className="h-11" /></Field><Field label="封面色系" htmlFor="cover-tone"><select id="cover-tone" value={form.coverTone} onChange={(event) => setField('coverTone', event.target.value as ProjectWriteInput['coverTone'])} className="h-11 w-full rounded-md border border-input bg-bg px-3 text-base"><option value="slate">岩灰</option><option value="amber">琥珀</option><option value="violet">紫罗兰</option><option value="cyan">青蓝</option><option value="emerald">翡翠</option></select></Field><Field label="封面关键词" htmlFor="cover-keywords" hint="最多三个，以逗号分隔"><Input id="cover-keywords" value={form.coverKeywords.join(', ')} onChange={(event) => setField('coverKeywords', event.target.value.split(/[,，]/).map((item) => item.trim()).filter(Boolean).slice(0, 3))} className="h-11" /></Field></div>}
              <p className="text-sm leading-6 text-text-muted">当前模式：{cover.coverResourceUrl ? '图片资源封面' : manualCover ? '手动文字封面' : 'Agent 文字封面'} · 置信度 {Math.round(cover.coverConfidence * 100)}%</p>
            </div>
          </div>
        </section>

        <section><Field label="锐评" htmlFor="project-critique"><Textarea id="project-critique" value={form.critique} onChange={(event) => setField('critique', event.target.value)} placeholder="一句直接、可行动的评价。" className="min-h-24 text-base" /></Field></section>

        <div className="flex items-center justify-between border-t border-border pt-6">
          <p className="text-base text-cat-internet" role="alert">{formError}</p>
          <Button type="submit" disabled={save.isPending}>{save.isPending ? (editing ? '正在保存…' : '正在上传…') : editing ? '保存修改' : '上传项目'}</Button>
        </div>
      </form>
    </div>
  )
}

function PageMessage({ children, error = false }: { children: string; error?: boolean }) { return <p className={`mx-auto max-w-4xl px-5 py-16 text-base ${error ? 'text-cat-internet' : 'text-text-muted'}`}>{children}</p> }

function SectionTitle({ title, description, action }: { title: string; description: string; action?: React.ReactNode }) { return <div className="flex flex-col gap-3 border-b border-border pb-3 sm:flex-row sm:items-end sm:justify-between"><div><h2 className="font-serif text-2xl font-semibold">{title}</h2><p className="mt-1 text-base leading-7 text-text-muted">{description}</p></div>{action}</div> }

function Field({ label, htmlFor, hint, className, children }: { label: string; htmlFor: string; hint?: string; className?: string; children: React.ReactNode }) { return <div className={className}><Label htmlFor={htmlFor} className="mb-2 block text-base">{label}</Label>{children}{hint && <p className="mt-1.5 text-sm text-text-faint">{hint}</p>}</div> }

function groupTags(tags: TagDefinition[]): Record<string, TagDefinition[]> {
  return tags.reduce<Record<string, TagDefinition[]>>((groups, tag) => {
    ;(groups[tag.groupName] ??= []).push(tag)
    return groups
  }, {})
}

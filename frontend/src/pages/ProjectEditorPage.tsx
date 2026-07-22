import { useEffect, useState } from 'react'
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { ArrowLeft, Plus, Trash2 } from 'lucide-react'
import { Link, useNavigate, useParams } from 'react-router-dom'
import { toast } from 'sonner'
import { createProject, getProject, updateProject } from '@/api/endpoints/projects'
import {
  ProjectWriteInputSchema,
  type ProjectResourceInput,
  type ProjectWriteInput,
} from '@/api/schemas/project'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { Label } from '@/components/ui/label'
import { Textarea } from '@/components/ui/textarea'
import { PROJECT_CATEGORIES } from '@/lib/projects'

const PROJECT_STATUSES = ['研发中', '运维测试', '迁移中', '已上线', '暂停维护', '已归档']
const RESOURCE_TYPES: { value: ProjectResourceInput['type']; label: string }[] = [
  { value: 'github', label: 'GitHub' },
  { value: 'baidu', label: '百度网盘' },
  { value: 'document', label: '文档 / PDF' },
  { value: 'archive', label: '压缩包' },
  { value: 'video', label: '展示视频' },
  { value: 'link', label: '其他链接' },
]

const EMPTY_PROJECT: ProjectWriteInput = {
  slug: '',
  name: '',
  summary: '',
  primaryCategory: '工具项目',
  highestAward: null,
  status: '研发中',
  critique: '',
  ownerName: null,
  sourceName: null,
  tags: [],
  resources: [],
}

export function ProjectEditorPage() {
  const { slug } = useParams()
  const editing = Boolean(slug)
  const navigate = useNavigate()
  const queryClient = useQueryClient()
  const [form, setForm] = useState<ProjectWriteInput>(EMPTY_PROJECT)
  const [tagsText, setTagsText] = useState('')
  const [formError, setFormError] = useState('')

  const project = useQuery({
    queryKey: ['project', slug],
    queryFn: () => getProject(slug!),
    enabled: editing,
  })

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
    })
    setTagsText(project.data.tags.join(', '))
  }, [project.data])

  const save = useMutation({
    mutationFn: (input: ProjectWriteInput) =>
      editing ? updateProject(slug!, input) : createProject(input),
    onSuccess: async (saved) => {
      await queryClient.invalidateQueries({ queryKey: ['projects'] })
      queryClient.setQueryData(['project', saved.slug], saved)
      toast.success(editing ? '项目已更新' : '项目已创建')
      navigate('/admin/projects')
    },
    onError: (error) => setFormError(error instanceof Error ? error.message : '保存失败'),
  })

  const submit = (event: React.FormEvent) => {
    event.preventDefault()
    const parsed = ProjectWriteInputSchema.safeParse({
      ...form,
      tags: tagsText
        .split(/[,，]/)
        .map((tag) => tag.trim())
        .filter(Boolean),
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

  if (editing && project.isLoading) {
    return <p className="mx-auto max-w-4xl px-5 py-16 text-sm text-text-muted">正在加载项目…</p>
  }
  if (editing && project.isError) {
    return <p className="mx-auto max-w-4xl px-5 py-16 text-sm text-cat-internet">项目加载失败。</p>
  }

  return (
    <div className="mx-auto max-w-4xl px-5 py-12 sm:px-8 sm:py-16">
      <Button asChild variant="ghost" size="sm" className="-ml-3">
        <Link to="/admin/projects">
          <ArrowLeft aria-hidden />
          返回项目管理
        </Link>
      </Button>
      <div className="mt-6 border-b border-border pb-7">
        <p className="text-xs font-medium uppercase tracking-[0.16em] text-text-faint">
          {editing ? 'Edit Project' : 'New Project'}
        </p>
        <h1 className="mt-3 font-serif text-3xl font-semibold">{editing ? '编辑项目' : '新建项目'}</h1>
      </div>

      <form onSubmit={submit} className="space-y-10 pt-8">
        <section className="grid gap-5 sm:grid-cols-2">
          <Field label="项目名" htmlFor="project-name" className="sm:col-span-2">
            <Input id="project-name" value={form.name} onChange={(event) => setField('name', event.target.value)} />
          </Field>
          <Field label="项目路径" htmlFor="project-slug" hint="用于 URL，例如 lab-device-booking">
            <Input
              id="project-slug"
              value={form.slug}
              onChange={(event) => setField('slug', event.target.value.toLowerCase())}
              placeholder="lowercase-slug"
            />
          </Field>
          <Field label="主要类别" htmlFor="project-category">
            <select
              id="project-category"
              value={form.primaryCategory}
              onChange={(event) =>
                setField('primaryCategory', event.target.value as ProjectWriteInput['primaryCategory'])
              }
              className="h-10 w-full rounded-md border border-input bg-bg px-3 text-sm shadow-sm focus:outline-none focus:ring-1 focus:ring-ring"
            >
              {PROJECT_CATEGORIES.map((category) => <option key={category}>{category}</option>)}
            </select>
          </Field>
          <Field label="内容简介" htmlFor="project-summary" className="sm:col-span-2">
            <Textarea
              id="project-summary"
              value={form.summary}
              onChange={(event) => setField('summary', event.target.value)}
              className="min-h-28"
            />
          </Field>
          <Field label="曾获奖" htmlFor="project-award">
            <Input
              id="project-award"
              value={form.highestAward ?? ''}
              onChange={(event) => setField('highestAward', event.target.value || null)}
              placeholder="没有可留空"
            />
          </Field>
          <Field label="目前状态" htmlFor="project-status">
            <select
              id="project-status"
              value={form.status}
              onChange={(event) => setField('status', event.target.value)}
              className="h-10 w-full rounded-md border border-input bg-bg px-3 text-sm shadow-sm focus:outline-none focus:ring-1 focus:ring-ring"
            >
              {PROJECT_STATUSES.map((status) => <option key={status}>{status}</option>)}
            </select>
          </Field>
          <Field label="目前负责" htmlFor="project-owner">
            <Input
              id="project-owner"
              value={form.ownerName ?? ''}
              onChange={(event) => setField('ownerName', event.target.value || null)}
            />
          </Field>
          <Field label="来源者 / 方" htmlFor="project-source">
            <Input
              id="project-source"
              value={form.sourceName ?? ''}
              onChange={(event) => setField('sourceName', event.target.value || null)}
            />
          </Field>
          <Field label="混合标签" htmlFor="project-tags" hint="使用逗号分隔，例如：软件, AI, Web" className="sm:col-span-2">
            <Input
              id="project-tags"
              value={tagsText}
              onChange={(event) => setTagsText(event.target.value)}
            />
          </Field>
          <Field label="锐评" htmlFor="project-critique" className="sm:col-span-2">
            <Textarea
              id="project-critique"
              value={form.critique}
              onChange={(event) => setField('critique', event.target.value)}
              placeholder="一句直接、可行动的评价。"
            />
          </Field>
        </section>

        <section>
          <div className="flex items-end justify-between border-b border-border pb-3">
            <div>
              <h2 className="font-serif text-xl font-semibold">项目资源</h2>
              <p className="mt-1 text-sm text-text-muted">GitHub、网盘、文档、压缩包或演示视频。</p>
            </div>
            <Button
              type="button"
              variant="outline"
              size="sm"
              onClick={() =>
                setField('resources', [...form.resources, { type: 'github', title: '', url: null }])
              }
            >
              <Plus aria-hidden />
              添加资源
            </Button>
          </div>
          <div className="space-y-3 pt-4">
            {form.resources.map((resource, index) => (
              <div key={index} className="grid gap-3 rounded-md border border-border p-3 sm:grid-cols-[140px_1fr_1.3fr_auto]">
                <select
                  value={resource.type}
                  onChange={(event) =>
                    setResource(index, { ...resource, type: event.target.value as ProjectResourceInput['type'] })
                  }
                  className="h-10 rounded-md border border-input bg-bg px-3 text-sm"
                  aria-label="资源类型"
                >
                  {RESOURCE_TYPES.map((type) => <option key={type.value} value={type.value}>{type.label}</option>)}
                </select>
                <Input
                  value={resource.title}
                  onChange={(event) => setResource(index, { ...resource, title: event.target.value })}
                  placeholder="资源标题"
                  aria-label="资源标题"
                />
                <Input
                  value={resource.url ?? ''}
                  onChange={(event) => setResource(index, { ...resource, url: event.target.value || null })}
                  placeholder="https://…（可稍后补）"
                  aria-label="资源链接"
                />
                <Button
                  type="button"
                  variant="ghost"
                  size="icon"
                  aria-label="移除资源"
                  onClick={() => setField('resources', form.resources.filter((_, itemIndex) => itemIndex !== index))}
                >
                  <Trash2 aria-hidden />
                </Button>
              </div>
            ))}
            {form.resources.length === 0 && (
              <p className="py-4 text-sm text-text-muted">暂不添加资源也可以，后续随时补充。</p>
            )}
          </div>
        </section>

        <div className="flex items-center justify-between border-t border-border pt-6">
          <p className="text-sm text-cat-internet" role="alert">{formError}</p>
          <Button type="submit" disabled={save.isPending}>
            {save.isPending ? '正在保存…' : editing ? '保存修改' : '创建项目'}
          </Button>
        </div>
      </form>
    </div>
  )
}

function Field({
  label,
  htmlFor,
  hint,
  className,
  children,
}: {
  label: string
  htmlFor: string
  hint?: string
  className?: string
  children: React.ReactNode
}) {
  return (
    <div className={className}>
      <Label htmlFor={htmlFor} className="mb-2 block">{label}</Label>
      {children}
      {hint && <p className="mt-1.5 text-xs text-text-faint">{hint}</p>}
    </div>
  )
}

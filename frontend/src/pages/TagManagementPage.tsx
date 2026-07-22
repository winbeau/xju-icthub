import { useState } from 'react'
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { ArrowLeft, Save } from 'lucide-react'
import { Link } from 'react-router-dom'
import { toast } from 'sonner'
import { listTags, mergeTag, updateTag } from '@/api/endpoints/tags'
import type { TagDefinition } from '@/api/schemas/tag'
import { canManageTags } from '@/api/schemas/user'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { AccessDeniedPage } from '@/pages/AccessDeniedPage'
import { useAuthStore } from '@/stores/authStore'

export function TagManagementPage() {
  const user = useAuthStore((state) => state.user)
  const queryClient = useQueryClient()
  const [drafts, setDrafts] = useState<Record<string, TagDefinition>>({})
  const tags = useQuery({ queryKey: ['tags', 'manage'], queryFn: () => listTags(true) })
  const save = useMutation({
    mutationFn: (tag: TagDefinition) =>
      updateTag(tag.id, {
        name: tag.name,
        groupName: tag.groupName,
        color: tag.color,
        sortOrder: tag.sortOrder,
        isActive: tag.isActive,
      }),
    onSuccess: async () => {
      await queryClient.invalidateQueries({ queryKey: ['tags'] })
      toast.success('标签已更新')
    },
    onError: (error) => toast.error(error instanceof Error ? error.message : '更新失败'),
  })
  const merge = useMutation({
    mutationFn: ({ id, targetId }: { id: string; targetId: string }) => mergeTag(id, targetId),
    onSuccess: async () => {
      await queryClient.invalidateQueries({ queryKey: ['tags'] })
      toast.success('同义标签已合并')
    },
    onError: (error) => toast.error(error instanceof Error ? error.message : '合并失败'),
  })

  if (!canManageTags(user)) return <AccessDeniedPage />
  const items = tags.data ?? []
  const draft = (tag: TagDefinition) => drafts[tag.id] ?? tag
  const patch = (tag: TagDefinition, values: Partial<TagDefinition>) =>
    setDrafts((current) => ({ ...current, [tag.id]: { ...draft(tag), ...values } }))

  return (
    <div className="mx-auto max-w-6xl px-5 py-12 sm:px-8 sm:py-16">
      <Button asChild variant="ghost" size="sm" className="-ml-3"><Link to="/admin/projects"><ArrowLeft aria-hidden />返回项目管理</Link></Button>
      <div className="mt-6 border-b border-border pb-7">
        <p className="text-sm font-medium uppercase tracking-[0.14em] text-text-faint">ICTHub / Tags</p>
        <h1 className="mt-3 font-serif text-4xl font-semibold">标签管理</h1>
        <p className="mt-3 text-base text-text-muted">重命名、调整分组和排序、停用或合并同义标签。</p>
      </div>
      <div className="mt-7 space-y-3">
        {items.map((tag) => {
          const value = draft(tag)
          return (
            <div key={tag.id} className="grid gap-3 rounded-lg border border-border p-4 lg:grid-cols-[1.2fr_150px_100px_100px_1fr_auto] lg:items-center">
              <Input value={value.name} onChange={(event) => patch(tag, { name: event.target.value })} aria-label="标签名称" />
              <select value={value.groupName} onChange={(event) => patch(tag, { groupName: event.target.value })} className="h-10 rounded-md border border-input bg-bg px-3 text-base" aria-label="标签分组"><option>比赛</option><option>技术</option><option>特征</option><option>领域</option><option>来源</option><option>历史</option></select>
              <Input type="number" value={value.sortOrder} onChange={(event) => patch(tag, { sortOrder: Number(event.target.value) })} aria-label="排序" />
              <label className="flex items-center gap-2 text-sm"><input type="checkbox" checked={value.isActive} onChange={(event) => patch(tag, { isActive: event.target.checked })} />启用</label>
              <select defaultValue="" onChange={(event) => { if (event.target.value && window.confirm(`确认将“${tag.name}”合并到目标标签？`)) merge.mutate({ id: tag.id, targetId: event.target.value }); event.currentTarget.value = '' }} className="h-10 rounded-md border border-input bg-bg px-3 text-sm" aria-label="合并到"><option value="">合并到…</option>{items.filter((target) => target.id !== tag.id && target.isActive).map((target) => <option key={target.id} value={target.id}>{target.name}</option>)}</select>
              <Button type="button" size="sm" variant="outline" disabled={save.isPending} onClick={() => save.mutate(value)}><Save aria-hidden />保存</Button>
            </div>
          )
        })}
      </div>
    </div>
  )
}

import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { PackageOpen, Pencil, Plus, Trash2 } from 'lucide-react'
import { Link } from 'react-router-dom'
import { toast } from 'sonner'
import { archiveProject, listProjects } from '@/api/endpoints/projects'
import { canManageTags } from '@/api/schemas/user'
import { Button } from '@/components/ui/button'
import { categoryColor } from '@/lib/projects'
import { useAuthStore } from '@/stores/authStore'

export function AdminProjectsPage() {
  const user = useAuthStore((state) => state.user)
  const queryClient = useQueryClient()
  const projects = useQuery({
    queryKey: ['projects', 'admin'],
    queryFn: () => listProjects({}),
  })

  const archive = useMutation({
    mutationFn: archiveProject,
    onSuccess: async () => {
      await queryClient.invalidateQueries({ queryKey: ['projects'] })
      toast.success('项目已归档')
    },
    onError: (error) => toast.error(error instanceof Error ? error.message : '归档失败'),
  })

  return (
    <div className="mx-auto max-w-5xl px-5 py-12 sm:px-8 sm:py-16">
      <div className="flex flex-col gap-5 border-b border-border pb-8 sm:flex-row sm:items-end sm:justify-between">
        <div>
          <p className="text-xs font-medium uppercase tracking-[0.16em] text-text-faint">
            Winbeau / Manage
          </p>
          <h1 className="mt-3 font-serif text-3xl font-semibold tracking-[-0.02em]">项目管理</h1>
          <p className="mt-2 text-base text-text-muted">上传、修订和归档个人项目。</p>
        </div>
        <div className="flex gap-2">
          {canManageTags(user) && <Button asChild variant="ghost"><Link to="/admin/tags">标签管理</Link></Button>}
          <Button asChild variant="outline">
            <Link to="/admin/import"><PackageOpen aria-hidden />一键导入</Link>
          </Button>
          <Button asChild>
            <Link to="/admin/projects/new">
              <Plus aria-hidden />
              上传项目
            </Link>
          </Button>
        </div>
      </div>

      <section aria-label="项目管理列表" aria-busy={projects.isLoading}>
        {projects.isLoading && <p className="py-12 text-sm text-text-muted">正在加载项目…</p>}
        {projects.isError && <p className="py-12 text-sm text-cat-internet">项目加载失败。</p>}
        {projects.data?.items.map((project) => (
          <article
            key={project.id}
            className="grid gap-3 border-b border-border py-5 sm:grid-cols-[minmax(0,1fr)_110px_auto] sm:items-center sm:gap-6"
          >
            <div className="min-w-0">
              <Link
                to={`/projects/${project.slug}`}
                className="font-serif text-xl font-semibold hover:underline"
              >
                {project.name}
              </Link>
              <p className="mt-1 line-clamp-1 text-sm text-text-muted">{project.summary}</p>
            </div>
            <div className="text-sm">
              <span className="block" style={{ color: categoryColor(project.primaryCategory) }}>
                {project.primaryCategory}
              </span>
              <span className="mt-1 block text-xs text-text-faint">{project.status}</span>
            </div>
            <div className="flex items-center gap-1">
              <Button asChild variant="ghost" size="sm">
                <Link to={`/admin/projects/${project.slug}/edit`}>
                  <Pencil aria-hidden />
                  编辑
                </Link>
              </Button>
              <Button
                variant="ghost"
                size="icon"
                aria-label={`归档 ${project.name}`}
                disabled={archive.isPending}
                onClick={() => {
                  if (window.confirm(`确认归档“${project.name}”？`)) archive.mutate(project.slug)
                }}
              >
                <Trash2 aria-hidden />
              </Button>
            </div>
          </article>
        ))}
        {projects.data?.items.length === 0 && (
          <p className="border-b border-border py-14 text-sm text-text-muted">
            还没有项目，可以手动上传或使用 Codex 一键导入。
          </p>
        )}
      </section>
    </div>
  )
}

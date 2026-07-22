import { useState } from 'react'
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { FileUp, PackageOpen, Pencil, Plus, Trash2 } from 'lucide-react'
import { Link } from 'react-router-dom'
import { toast } from 'sonner'
import { archiveProject, importProjects, listProjects } from '@/api/endpoints/projects'
import type { ProjectWriteInput } from '@/api/schemas/project'
import { canManageTags } from '@/api/schemas/user'
import { Button } from '@/components/ui/button'
import { Textarea } from '@/components/ui/textarea'
import { categoryColor } from '@/lib/projects'
import { parseProjectImport } from '@/lib/projectImport'
import { useAuthStore } from '@/stores/authStore'

const IMPORT_EXAMPLE = `项目路径\t项目名\t类别\t简介\t曾获奖\t状态\t负责人\t来源\t标签\tgh链接\t百度网盘链接
lab-portal\t实验室门户\t传统软件\t统一实验室日常入口。\t\t研发中\t基础设施组\t内部需求\tWeb|实验室建设\thttps://github.com/example/repo\t`

export function AdminProjectsPage() {
  const user = useAuthStore((state) => state.user)
  const queryClient = useQueryClient()
  const [showImport, setShowImport] = useState(false)
  const [importText, setImportText] = useState('')
  const [importPreview, setImportPreview] = useState<ProjectWriteInput[] | null>(null)
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

  const importMutation = useMutation({
    mutationFn: importProjects,
    onSuccess: async (result) => {
      await queryClient.invalidateQueries({ queryKey: ['projects'] })
      setImportText('')
      setImportPreview(null)
      toast.success(`导入完成：新增 ${result.created}，更新 ${result.updated}`)
    },
    onError: (error) => toast.error(error instanceof Error ? error.message : '导入失败'),
  })

  const previewImport = () => {
    try {
      const items = parseProjectImport(importText)
      setImportPreview(items)
    } catch (error) {
      toast.error(error instanceof Error ? error.message : '无法解析导入内容')
    }
  }

  return (
    <div className="mx-auto max-w-5xl px-5 py-12 sm:px-8 sm:py-16">
      <div className="flex flex-col gap-5 border-b border-border pb-8 sm:flex-row sm:items-end sm:justify-between">
        <div>
          <p className="text-xs font-medium uppercase tracking-[0.16em] text-text-faint">
            ICTHub / Manage
          </p>
          <h1 className="mt-3 font-serif text-3xl font-semibold tracking-[-0.02em]">项目管理</h1>
          <p className="mt-2 text-base text-text-muted">上传、修订和归档实验室项目。</p>
        </div>
        <div className="flex gap-2">
          {canManageTags(user) && <Button asChild variant="ghost"><Link to="/admin/tags">标签管理</Link></Button>}
          <Button asChild variant="outline">
            <Link to="/admin/import"><PackageOpen aria-hidden />一键导入</Link>
          </Button>
          <Button variant="outline" onClick={() => setShowImport((value) => !value)}>
            <FileUp aria-hidden />
            快速导入
          </Button>
          <Button asChild>
            <Link to="/admin/projects/new">
              <Plus aria-hidden />
              上传项目
            </Link>
          </Button>
        </div>
      </div>

      {showImport && (
        <section className="border-b border-border py-7" aria-label="快速导入">
          <div className="flex flex-col gap-4 md:flex-row md:items-start md:justify-between">
            <div className="max-w-2xl">
              <h2 className="font-medium">从表格粘贴</h2>
              <p className="mt-1 text-sm leading-6 text-text-muted">
                支持 Excel/WPS 复制出的制表符表格，也支持 CSV。相同 slug 会更新原项目；单次最多 200
                条。
              </p>
            </div>
            <Button
              variant="ghost"
              size="sm"
              onClick={() => {
                setImportText(IMPORT_EXAMPLE)
                setImportPreview(null)
              }}
            >
              填入示例
            </Button>
          </div>
          <Textarea
            value={importText}
            onChange={(event) => {
              setImportText(event.target.value)
              setImportPreview(null)
            }}
            placeholder="项目路径    项目名    类别    简介……"
            className="mt-4 min-h-40 font-mono text-xs leading-6"
          />
          {importPreview && (
            <div className="mt-4 rounded-md border border-border bg-bg-subtle p-4">
              <p className="text-sm font-medium">准备导入 {importPreview.length} 个项目</p>
              <ul className="mt-2 space-y-1 text-sm text-text-muted">
                {importPreview.slice(0, 5).map((project) => (
                  <li key={project.slug}>
                    {project.name} · {project.primaryCategory}
                  </li>
                ))}
              </ul>
              {importPreview.length > 5 && (
                <p className="mt-2 text-xs text-text-faint">
                  另有 {importPreview.length - 5} 个项目
                </p>
              )}
            </div>
          )}
          <div className="mt-3 flex justify-end gap-2">
            {importPreview ? (
              <>
                <Button variant="ghost" onClick={() => setImportPreview(null)}>
                  重新检查
                </Button>
                <Button
                  onClick={() => importMutation.mutate(importPreview)}
                  disabled={importMutation.isPending}
                >
                  {importMutation.isPending ? '正在导入…' : '确认导入'}
                </Button>
              </>
            ) : (
              <Button onClick={previewImport} disabled={!importText.trim()}>
                解析并预览
              </Button>
            )}
          </div>
        </section>
      )}

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
            还没有项目，可以上传或从表格导入。
          </p>
        )}
      </section>
    </div>
  )
}

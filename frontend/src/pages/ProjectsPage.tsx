import { useDeferredValue, useState } from 'react'
import { useQuery } from '@tanstack/react-query'
import { ArrowRight, Search, Sparkles, Trophy } from 'lucide-react'
import { Link } from 'react-router-dom'
import { listProjects } from '@/api/endpoints/projects'
import type { ProjectCategory } from '@/api/schemas/project'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { PROJECT_CATEGORIES, categoryColor } from '@/lib/projects'

export function ProjectsPage() {
  const [search, setSearch] = useState('')
  const [category, setCategory] = useState<ProjectCategory | undefined>()
  const [agentQuestion, setAgentQuestion] = useState('')
  const [agentAnswer, setAgentAnswer] = useState('例如：找一个适合改造成互联网+项目的工具。')
  const deferredSearch = useDeferredValue(search)

  const projects = useQuery({
    queryKey: ['projects', deferredSearch, category],
    queryFn: () => listProjects({ q: deferredSearch || undefined, category }),
  })

  const askAgent = () => {
    const question = agentQuestion.trim()
    if (!question) {
      setAgentAnswer('先输入一个问题，我会从项目名称、简介、类别和获奖中检索。')
    } else if (question.includes('论文')) {
      setAgentAnswer(
        '可以先查看“面向边缘设备的轻量目标检测研究”，工具项目也可继续挖掘可复现性方向。',
      )
    } else if (question.includes('互联网+')) {
      setAgentAnswer('“实验室设备预约系统”具备真实场景，但需要补充使用数据、服务对象与推广路径。')
    } else {
      setAgentAnswer('Agent 接口槽位已保留；接入 Rust 检索工具后会返回带项目引用的答案。')
    }
  }

  return (
    <div className="mx-auto max-w-6xl px-5 py-12 sm:px-8 sm:py-16">
      <section className="grid gap-9 md:grid-cols-[minmax(0,1fr)_320px] md:items-end">
        <div>
          <p className="text-sm font-medium uppercase tracking-[0.16em] text-text-faint">
            ICTHub / Projects
          </p>
          <h1 className="mt-4 font-serif text-3xl font-semibold leading-tight tracking-[-0.02em] sm:text-[42px]">
            实验室项目集
          </h1>
          <p className="mt-4 max-w-2xl text-lg leading-8 text-text-muted">
            记录我们的探索、实践与成果。
          </p>
        </div>

        <label className="block">
          <span className="mb-2 block text-sm font-medium text-text-muted">搜索项目</span>
          <span className="relative block">
            <Search
              size={16}
              className="pointer-events-none absolute left-3 top-1/2 -translate-y-1/2 text-text-faint"
              aria-hidden
            />
            <Input
              value={search}
              onChange={(event) => setSearch(event.target.value)}
              placeholder="项目名称、简介或获奖"
              className="h-10 pl-9"
            />
          </span>
        </label>
      </section>

      <div
        className="mt-10 flex flex-wrap gap-x-6 gap-y-3 border-b border-border pb-4"
        aria-label="项目类别"
      >
        <CategoryButton active={!category} onClick={() => setCategory(undefined)}>
          全部项目
        </CategoryButton>
        {PROJECT_CATEGORIES.map((item) => (
          <CategoryButton key={item} active={category === item} onClick={() => setCategory(item)}>
            {item}
          </CategoryButton>
        ))}
      </div>

      <section className="min-h-72" aria-label="项目目录" aria-busy={projects.isLoading}>
        {projects.isLoading && (
          <div className="space-y-0">
            {[1, 2, 3].map((item) => (
              <div
                key={item}
                className="grid animate-pulse gap-4 border-b border-border py-7 md:grid-cols-[150px_1fr_24px]"
              >
                <div className="h-4 w-24 rounded bg-bg-subtle" />
                <div>
                  <div className="h-6 w-2/5 rounded bg-bg-subtle" />
                  <div className="mt-3 h-4 w-4/5 rounded bg-bg-subtle" />
                </div>
              </div>
            ))}
          </div>
        )}

        {projects.isError && (
          <div className="border-b border-border py-12 text-sm text-cat-internet">
            项目加载失败，请稍后重试。
          </div>
        )}

        {projects.data?.items.map((project) => (
          <Link
            key={project.id}
            to={`/projects/${project.slug}`}
            className="group grid gap-3 border-b border-border py-7 transition-colors hover:bg-bg-subtle/70 md:grid-cols-[150px_minmax(0,1fr)_24px] md:gap-6 md:px-2"
          >
            <span
              className="text-base font-medium"
              style={{ color: categoryColor(project.primaryCategory) }}
            >
              {project.primaryCategory}
            </span>
            <div>
              <h2 className="font-serif text-xl font-semibold leading-snug tracking-[-0.01em] text-text sm:text-2xl">
                {project.name}
              </h2>
              <p className="mt-1.5 max-w-3xl leading-6 text-text-muted">{project.summary}</p>
              <div className="mt-3 flex items-center gap-1.5 text-base text-text-muted">
                <Trophy size={15} strokeWidth={1.7} aria-hidden />
                <span>曾获奖：{project.highestAward ?? '暂无'}</span>
              </div>
            </div>
            <ArrowRight
              size={18}
              className="mt-1 hidden text-text-faint transition-transform group-hover:translate-x-1 md:block"
              aria-hidden
            />
          </Link>
        ))}

        {projects.data && projects.data.items.length === 0 && (
          <div className="border-b border-border py-14 text-base text-text-muted">
            没有匹配项目。
          </div>
        )}
      </section>

      <section className="mt-12 border-t border-border pt-8" aria-label="Agent 问答预留">
        <div className="flex flex-col gap-3 sm:flex-row sm:items-center">
          <Sparkles size={18} className="shrink-0 text-text-muted" aria-hidden />
          <Input
            value={agentQuestion}
            onChange={(event) => setAgentQuestion(event.target.value)}
            onKeyDown={(event) => {
              if (event.key === 'Enter') askAgent()
            }}
            placeholder="让 Agent 帮你找项目"
            className="max-w-xl"
          />
          <Button variant="outline" onClick={askAgent}>
            提问
          </Button>
        </div>
        <p className="mt-3 max-w-3xl text-base leading-7 text-text-muted sm:pl-8">{agentAnswer}</p>
      </section>
    </div>
  )
}

function CategoryButton({
  active,
  children,
  onClick,
}: {
  active: boolean
  children: string
  onClick: () => void
}) {
  return (
    <button
      type="button"
      aria-pressed={active}
      onClick={onClick}
      className={`border-b pb-1 text-base transition-colors ${
        active
          ? 'border-text font-medium text-text'
          : 'border-transparent text-text-muted hover:text-text'
      }`}
    >
      {children}
    </button>
  )
}

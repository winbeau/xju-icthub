import { Bot, PenLine } from 'lucide-react'
import { Link } from 'react-router-dom'
import { cn } from '@/lib/cn'

export function CreateModeSwitch({ mode }: { mode: 'manual' | 'codex' }) {
  return (
    <nav
      aria-label="项目上传方式"
      className="inline-flex rounded-lg border border-border bg-bg-subtle p-1"
    >
      <ModeLink active={mode === 'manual'} to="/admin/projects/new">
        <PenLine aria-hidden />手动填写
      </ModeLink>
      <ModeLink active={mode === 'codex'} to="/admin/import">
        <Bot aria-hidden />Codex 整理
      </ModeLink>
    </nav>
  )
}

function ModeLink({
  active,
  children,
  to,
}: {
  active: boolean
  children: React.ReactNode
  to: string
}) {
  return (
    <Link
      aria-current={active ? 'page' : undefined}
      className={cn(
        'flex h-9 items-center gap-2 rounded-md px-3 text-sm font-medium transition-colors',
        active
          ? 'bg-bg text-text shadow-sm'
          : 'text-text-muted hover:bg-bg/70 hover:text-text',
      )}
      to={to}
    >
      {children}
    </Link>
  )
}

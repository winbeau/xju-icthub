import type { ProjectCover } from '@/api/schemas/project'
import { cn } from '@/lib/cn'
import { isImageUrl } from '@/lib/covers'

const toneClasses: Record<ProjectCover['coverTone'], string> = {
  slate: 'from-slate-100 to-stone-200 text-slate-800',
  amber: 'from-amber-100 to-orange-200 text-amber-950',
  violet: 'from-violet-100 to-fuchsia-200 text-violet-950',
  cyan: 'from-cyan-100 to-sky-200 text-cyan-950',
  emerald: 'from-emerald-100 to-teal-200 text-emerald-950',
}

export function ProjectCover({
  cover,
  className,
}: {
  cover: ProjectCover
  className?: string
}) {
  if (cover.coverResourceUrl && isImageUrl(cover.coverResourceUrl)) {
    return (
      <div className={cn('overflow-hidden rounded-lg bg-bg-subtle', className)}>
        <img
          src={cover.coverResourceUrl}
          alt={`${cover.coverTitle}项目封面`}
          className="h-full w-full object-cover"
        />
      </div>
    )
  }

  return (
    <div
      className={cn(
        'flex flex-col justify-between rounded-lg bg-gradient-to-br p-5',
        toneClasses[cover.coverTone],
        className,
      )}
    >
      <div>
        <p className="text-2xl font-semibold tracking-[-0.03em]">{cover.coverTitle}</p>
        <p className="mt-2 line-clamp-2 text-sm leading-6 opacity-75">{cover.coverSubtitle}</p>
      </div>
      <div className="mt-5 flex flex-wrap gap-1.5">
        {cover.coverKeywords.slice(0, 3).map((keyword) => (
          <span key={keyword} className="rounded-full bg-white/55 px-2 py-0.5 text-xs">
            {keyword}
          </span>
        ))}
      </div>
    </div>
  )
}

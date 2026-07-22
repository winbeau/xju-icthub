import type { ProjectCategory, ProjectSummary } from '@/api/schemas/project'

export const PROJECT_CATEGORIES: readonly ProjectCategory[] = [
  '传统软件',
  '智能硬件',
  'AI 软件',
  '数字媒体',
  '研究成果',
]

export function filterProjects(
  projects: readonly ProjectSummary[],
  q?: string,
  category?: ProjectCategory,
): ProjectSummary[] {
  const needle = q?.trim().toLocaleLowerCase('zh-CN') ?? ''
  return projects.filter((project) => {
    const matchesCategory = !category || project.primaryCategory === category
    const haystack = `${project.name} ${project.summary} ${project.highestAward ?? ''}`
      .toLocaleLowerCase('zh-CN')
      .trim()
    return matchesCategory && (!needle || haystack.includes(needle))
  })
}

export function categoryColor(category: ProjectCategory): string {
  switch (category) {
    case '传统软件':
      return 'var(--cat-software)'
    case '智能硬件':
      return 'var(--cat-hardware)'
    case 'AI 软件':
      return 'var(--cat-ai)'
    case '数字媒体':
      return 'var(--cat-media)'
    case '研究成果':
      return 'var(--cat-research)'
  }
}

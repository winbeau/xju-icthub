import type { ProjectCategory, ProjectSummary } from '@/api/schemas/project'

export const PROJECT_CATEGORIES: readonly ProjectCategory[] = [
  '互联网+',
  '计算机设计大赛',
  '论文',
  '工具项目',
  '其他',
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
    case '互联网+':
      return 'var(--cat-internet)'
    case '计算机设计大赛':
      return 'var(--cat-design)'
    case '论文':
      return 'var(--cat-paper)'
    case '工具项目':
      return 'var(--cat-tools)'
    case '其他':
      return 'var(--cat-other)'
  }
}

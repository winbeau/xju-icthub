import type {
  ProjectCategory,
  ProjectCover,
  ProjectResourceInput,
} from '@/api/schemas/project'

const TONES: Record<ProjectCategory, ProjectCover['coverTone']> = {
  传统软件: 'slate',
  智能硬件: 'amber',
  'AI 软件': 'violet',
  数字媒体: 'cyan',
  研究成果: 'emerald',
}

export function buildCoverPreview(input: {
  name: string
  summary: string
  primaryCategory: ProjectCategory
  tags: string[]
  resources: ProjectResourceInput[]
}): ProjectCover {
  const image = input.resources.find(
    (resource) => resource.type === 'image' || isImageUrl(resource.url),
  )
  return {
    coverMode: image ? 'resource' : 'text',
    coverResourceId: null,
    coverResourceUrl: image?.url ?? null,
    coverTitle: fallbackTitle(input.name, input.primaryCategory),
    coverSubtitle: [...input.summary.trim()].slice(0, 20).join(''),
    coverKeywords: input.tags.slice(0, 3).length
      ? input.tags.slice(0, 3)
      : [input.primaryCategory],
    coverTone: TONES[input.primaryCategory],
    coverConfidence: image ? 0.92 : 0.74,
  }
}

export function isImageUrl(url: string | null | undefined): boolean {
  if (!url) return false
  return /\.(?:png|jpe?g|webp|gif|avif)(?:[?#].*)?$/i.test(url)
}

function fallbackTitle(name: string, category: ProjectCategory): string {
  const curated: [string, string][] = [
    ['棉花', '棉田智检'],
    ['病虫害', '病害智检'],
    ['机器人', '智能巡检'],
    ['会议纪要', '智能纪要'],
    ['预约', '便捷预约'],
    ['归档', '实验归档'],
    ['预测', '智能预测'],
  ]
  const matched = curated.find(([keyword]) => name.includes(keyword))
  if (matched) return matched[1]
  const cleaned = name.replace(/基于|面向|平台|系统|项目/g, '').replace(/[^\u4e00-\u9fff]/g, '')
  const title = [...cleaned].slice(0, 8).join('')
  if ([...title].length >= 4) return title
  return {
    传统软件: '软件项目',
    智能硬件: '智能硬件',
    'AI 软件': '智能应用',
    数字媒体: '数字创意',
    研究成果: '研究成果',
  }[category]
}

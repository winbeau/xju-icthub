import { describe, expect, it } from 'vitest'
import { PROJECT_FIXTURES } from '@/api/mock/fixtures'
import { filterProjects, PROJECT_CATEGORIES } from '@/lib/projects'

describe('filterProjects', () => {
  it('exposes only the five formal categories', () => {
    expect(PROJECT_CATEGORIES).toEqual(['传统软件', '智能硬件', 'AI 软件', '数字媒体', '研究成果'])
  })
  it('filters by category', () => {
    const result = filterProjects(PROJECT_FIXTURES, '', '研究成果')
    expect(result).toHaveLength(1)
    expect(result[0]?.name).toContain('目标检测')
  })

  it('searches name, summary and award', () => {
    expect(filterProjects(PROJECT_FIXTURES, '国赛二等奖')).toHaveLength(1)
    expect(filterProjects(PROJECT_FIXTURES, '二维码')).toHaveLength(1)
    expect(filterProjects(PROJECT_FIXTURES, '能耗')).toHaveLength(1)
  })
})

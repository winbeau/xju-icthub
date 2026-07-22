import { describe, expect, it } from 'vitest'
import { PROJECT_FIXTURES } from '@/api/mock/fixtures'
import { filterProjects } from '@/lib/projects'

describe('filterProjects', () => {
  it('filters by category', () => {
    const result = filterProjects(PROJECT_FIXTURES, '', '论文')
    expect(result).toHaveLength(1)
    expect(result[0]?.name).toContain('目标检测')
  })

  it('searches name, summary and award', () => {
    expect(filterProjects(PROJECT_FIXTURES, '国赛二等奖')).toHaveLength(1)
    expect(filterProjects(PROJECT_FIXTURES, '二维码')).toHaveLength(1)
    expect(filterProjects(PROJECT_FIXTURES, '会议纪要')).toHaveLength(1)
  })
})

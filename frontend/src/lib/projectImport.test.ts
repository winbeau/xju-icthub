import { describe, expect, it } from 'vitest'
import { parseProjectImport } from '@/lib/projectImport'

describe('parseProjectImport', () => {
  it('parses a tab-separated project sheet', () => {
    const projects = parseProjectImport(
      '项目路径\t项目名\t类别\t简介\t曾获奖\t标签\tgh链接\nlab-tool\t实验室工具\t工具项目\t日常使用的工具。\t校赛一等奖\t软件|Web\thttps://github.com/example/repo',
    )
    expect(projects).toHaveLength(1)
    expect(projects[0]).toMatchObject({
      slug: 'lab-tool',
      primaryCategory: '工具项目',
      highestAward: '校赛一等奖',
      tags: ['软件', 'Web'],
    })
    expect(projects[0]?.resources[0]?.type).toBe('github')
  })

  it('reports a missing required header', () => {
    expect(() => parseProjectImport('项目名,类别,简介\n工具,工具项目,说明')).toThrow('缺少必需列')
  })

  it('reports the row number for invalid data', () => {
    expect(() =>
      parseProjectImport('slug,name,category,summary\nbad,项目,未知类别,说明'),
    ).toThrow('第 2 行')
  })
})

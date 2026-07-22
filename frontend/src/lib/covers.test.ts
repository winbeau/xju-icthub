import { describe, expect, it } from 'vitest'
import { buildCoverPreview } from '@/lib/covers'

describe('buildCoverPreview', () => {
  it('generates a deterministic text cover without an image', () => {
    const cover = buildCoverPreview({
      name: '实验室设备预约系统',
      summary: '管理设备档案、预约冲突和借用交接。',
      primaryCategory: '传统软件',
      tags: ['Web', '校园服务'],
      resources: [],
    })
    expect(cover.coverMode).toBe('text')
    expect([...cover.coverTitle].length).toBeGreaterThanOrEqual(4)
    expect([...cover.coverTitle].length).toBeLessThanOrEqual(8)
    expect([...cover.coverSubtitle].length).toBeLessThanOrEqual(20)
  })

  it('prefers an image resource over the text cover', () => {
    const cover = buildCoverPreview({
      name: '项目截图测试',
      summary: '验证图片资源识别。',
      primaryCategory: '数字媒体',
      tags: ['3D/VR'],
      resources: [
        { type: 'video', title: '展示视频', url: 'https://example.com/demo.mp4' },
        { type: 'link', title: '项目截图', url: 'https://example.com/cover.webp' },
      ],
    })
    expect(cover.coverMode).toBe('resource')
    expect(cover.coverResourceUrl).toBe('https://example.com/cover.webp')
  })
})

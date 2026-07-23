import { describe, expect, it } from 'vitest'
import { ProjectResourceSchema, ProjectWriteInputSchema } from '@/api/schemas/project'

describe('project resource schemas', () => {
  it('accepts persisted preview metadata returned by the backend', () => {
    const resource = ProjectResourceSchema.parse({
      id: 'resource-1',
      type: 'document',
      title: '项目说明书',
      url: null,
      sourceName: '说明书.pdf',
      mimeType: 'application/pdf',
      sizeBytes: 2048,
      displayPath: 'docs/说明书.pdf',
      previewKind: 'pdf',
      contentUrl: '/api/v1/projects/demo/resources/resource-1/content',
      downloadUrl: '/api/v1/projects/demo/resources/resource-1/download',
    })
    expect(resource.previewKind).toBe('pdf')
    expect(resource.contentUrl).toContain('/content')
  })

  it('keeps import artifact references in an upload draft', () => {
    const project = ProjectWriteInputSchema.parse({
      slug: 'import-demo',
      name: '导入测试',
      summary: '验证导入任务附件引用。',
      primaryCategory: '传统软件',
      highestAward: null,
      status: '研发中',
      critique: '',
      ownerName: null,
      sourceName: null,
      tags: [],
      resources: [
        {
          type: 'presentation',
          title: 'HTML 演示',
          url: null,
          sourceImportJobId: 'job-1',
          sourceArtifactId: 'artifact-1',
        },
      ],
    })
    expect(project.resources[0]?.sourceArtifactId).toBe('artifact-1')
  })
})

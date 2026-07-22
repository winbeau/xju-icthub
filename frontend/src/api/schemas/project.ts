import { z } from 'zod'

export const ProjectCategorySchema = z.enum([
  '互联网+',
  '计算机设计大赛',
  '论文',
  '工具项目',
  '其他',
])

export type ProjectCategory = z.infer<typeof ProjectCategorySchema>

export const ProjectSummarySchema = z.object({
  id: z.string(),
  slug: z.string(),
  name: z.string(),
  summary: z.string(),
  primaryCategory: ProjectCategorySchema,
  highestAward: z.string().nullable(),
})

export type ProjectSummary = z.infer<typeof ProjectSummarySchema>

export const ProjectResourceSchema = z.object({
  id: z.string(),
  type: z.enum(['github', 'baidu', 'document', 'archive', 'video', 'link']),
  title: z.string(),
  url: z.string().url().nullable(),
})

export type ProjectResource = z.infer<typeof ProjectResourceSchema>

export const ProjectDetailSchema = ProjectSummarySchema.extend({
  status: z.string(),
  critique: z.string(),
  ownerName: z.string().nullable(),
  sourceName: z.string().nullable(),
  tags: z.array(z.string()),
  resources: z.array(ProjectResourceSchema),
})

export type ProjectDetail = z.infer<typeof ProjectDetailSchema>

export const ProjectListResponseSchema = z.object({
  items: z.array(ProjectSummarySchema),
  total: z.number().int().nonnegative(),
})

export type ProjectListResponse = z.infer<typeof ProjectListResponseSchema>

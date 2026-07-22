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
  status: z.string(),
})

export type ProjectSummary = z.infer<typeof ProjectSummarySchema>

export const ProjectResourceSchema = z.object({
  id: z.string(),
  type: z.enum(['github', 'baidu', 'document', 'archive', 'video', 'link']),
  title: z.string(),
  url: z.string().url().nullable(),
})

export type ProjectResource = z.infer<typeof ProjectResourceSchema>

export const ProjectResourceInputSchema = ProjectResourceSchema.omit({ id: true })

export type ProjectResourceInput = z.infer<typeof ProjectResourceInputSchema>

export const ProjectDetailSchema = ProjectSummarySchema.extend({
  status: z.string(),
  critique: z.string(),
  ownerName: z.string().nullable(),
  sourceName: z.string().nullable(),
  tags: z.array(z.string()),
  resources: z.array(ProjectResourceSchema),
})

export type ProjectDetail = z.infer<typeof ProjectDetailSchema>

export const ProjectWriteInputSchema = z.object({
  slug: z
    .string()
    .trim()
    .min(1, '请填写项目路径')
    .max(80)
    .regex(/^[a-z0-9]+(?:-[a-z0-9]+)*$/, '只能使用小写字母、数字和连字符'),
  name: z.string().trim().min(1, '请填写项目名').max(120),
  summary: z.string().trim().min(1, '请填写项目简介').max(500),
  primaryCategory: ProjectCategorySchema,
  highestAward: z.string().trim().nullable(),
  status: z.string().trim().min(1, '请填写项目状态'),
  critique: z.string().trim(),
  ownerName: z.string().trim().nullable(),
  sourceName: z.string().trim().nullable(),
  tags: z.array(z.string().trim().min(1)).max(20),
  resources: z.array(ProjectResourceInputSchema),
})

export type ProjectWriteInput = z.infer<typeof ProjectWriteInputSchema>

export const ProjectImportResponseSchema = z.object({
  created: z.number().int().nonnegative(),
  updated: z.number().int().nonnegative(),
  total: z.number().int().nonnegative(),
})

export type ProjectImportResponse = z.infer<typeof ProjectImportResponseSchema>

export const ProjectListResponseSchema = z.object({
  items: z.array(ProjectSummarySchema),
  total: z.number().int().nonnegative(),
})

export type ProjectListResponse = z.infer<typeof ProjectListResponseSchema>

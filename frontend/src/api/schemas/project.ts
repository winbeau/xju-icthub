import { z } from 'zod'

export const ProjectCategorySchema = z.enum([
  '传统软件',
  '智能硬件',
  'AI 软件',
  '数字媒体',
  '研究成果',
])

export type ProjectCategory = z.infer<typeof ProjectCategorySchema>

export const CoverModeSchema = z.enum(['manual', 'resource', 'text'])
export const CoverToneSchema = z.enum(['slate', 'amber', 'violet', 'cyan', 'emerald'])

export const ProjectCoverSchema = z.object({
  coverMode: CoverModeSchema,
  coverResourceId: z.string().nullable(),
  coverResourceUrl: z.string().url().nullable(),
  coverTitle: z.string(),
  coverSubtitle: z.string(),
  coverKeywords: z.array(z.string()).max(3),
  coverTone: CoverToneSchema,
  coverConfidence: z.number().min(0).max(1),
})

export type ProjectCover = z.infer<typeof ProjectCoverSchema>

export const ProjectSummarySchema = z
  .object({
    id: z.string(),
    slug: z.string(),
    name: z.string(),
    summary: z.string(),
    primaryCategory: ProjectCategorySchema,
    highestAward: z.string().nullable(),
    status: z.string(),
    tags: z.array(z.string()),
  })
  .merge(ProjectCoverSchema)

export type ProjectSummary = z.infer<typeof ProjectSummarySchema>

export const ProjectResourceSchema = z.object({
  id: z.string(),
  type: z.enum([
    'github',
    'baidu',
    'document',
    'presentation',
    'archive',
    'video',
    'image',
    'link',
  ]),
  title: z.string(),
  url: z.string().url().nullable(),
})

export type ProjectResource = z.infer<typeof ProjectResourceSchema>
export const ProjectResourceInputSchema = ProjectResourceSchema.omit({ id: true })
export type ProjectResourceInput = z.infer<typeof ProjectResourceInputSchema>

export const ProjectDetailSchema = ProjectSummarySchema.extend({
  classificationStatus: z.enum(['classified', 'pending']),
  status: z.string(),
  critique: z.string(),
  ownerName: z.string().nullable(),
  sourceName: z.string().nullable(),
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
  coverMode: CoverModeSchema.default('text'),
  coverTitle: z.string().trim().nullable().default(null),
  coverSubtitle: z.string().trim().nullable().default(null),
  coverKeywords: z.array(z.string().trim().min(1)).max(3).default([]),
  coverTone: CoverToneSchema.default('slate'),
})

export type ProjectWriteInput = z.infer<typeof ProjectWriteInputSchema>

export const GeneratedCoverSchema = ProjectCoverSchema
export type GeneratedCover = z.infer<typeof GeneratedCoverSchema>

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

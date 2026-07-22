import { z } from 'zod'

export const TagDefinitionSchema = z.object({
  id: z.string(),
  name: z.string(),
  groupName: z.string(),
  color: z.string().nullable(),
  sortOrder: z.number().int(),
  isActive: z.boolean(),
  mergedIntoId: z.string().nullable(),
})

export type TagDefinition = z.infer<typeof TagDefinitionSchema>

export const TagCreateInputSchema = z.object({
  name: z.string().trim().min(1).max(40),
  groupName: z.string().trim().min(1).max(40),
  color: z.string().nullable(),
  sortOrder: z.number().int(),
})

export type TagCreateInput = z.infer<typeof TagCreateInputSchema>

export const TagUpdateInputSchema = z.object({
  name: z.string().trim().min(1).max(40).optional(),
  groupName: z.string().trim().min(1).max(40).optional(),
  color: z.string().nullable().optional(),
  sortOrder: z.number().int().optional(),
  isActive: z.boolean().optional(),
})

export type TagUpdateInput = z.infer<typeof TagUpdateInputSchema>

export const TagSuggestionResponseSchema = z.object({
  id: z.string(),
  status: z.literal('pending'),
})

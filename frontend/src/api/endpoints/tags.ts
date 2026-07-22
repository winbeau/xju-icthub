import { z } from 'zod'
import { request } from '@/api/client'
import { authHeaders } from '@/api/endpoints/auth'
import {
  TagCreateInputSchema,
  TagDefinitionSchema,
  TagSuggestionResponseSchema,
  TagUpdateInputSchema,
  type TagCreateInput,
  type TagDefinition,
  type TagUpdateInput,
} from '@/api/schemas/tag'

export function listTags(includeInactive = false): Promise<TagDefinition[]> {
  return request({
    method: 'GET',
    path: '/api/v1/tags',
    query: { includeInactive },
    schema: z.array(TagDefinitionSchema),
    headers: authHeaders(),
  })
}

export function updateTag(id: string, input: TagUpdateInput): Promise<TagDefinition> {
  return request({
    method: 'PATCH',
    path: `/api/v1/tags/${encodeURIComponent(id)}`,
    body: TagUpdateInputSchema.parse(input),
    schema: TagDefinitionSchema,
    headers: authHeaders(),
  })
}

export function mergeTag(id: string, targetId: string): Promise<null> {
  return request({
    method: 'POST',
    path: `/api/v1/tags/${encodeURIComponent(id)}/merge`,
    body: { targetId },
    schema: z.null(),
    headers: authHeaders(),
  })
}

export function createTag(input: TagCreateInput): Promise<TagDefinition> {
  return request({
    method: 'POST',
    path: '/api/v1/tags',
    body: TagCreateInputSchema.parse(input),
    schema: TagDefinitionSchema,
    headers: authHeaders(),
  })
}

export function suggestTag(input: {
  name: string
  groupName: string | null
  reason: string | null
}) {
  return request({
    method: 'POST',
    path: '/api/v1/tag-suggestions',
    body: input,
    schema: TagSuggestionResponseSchema,
    headers: authHeaders(),
  })
}

import { request } from '@/api/client'
import { authHeaders } from '@/api/endpoints/auth'
import {
  ProjectDetailSchema,
  ProjectImportResponseSchema,
  ProjectListResponseSchema,
  type ProjectCategory,
  type ProjectDetail,
  type ProjectImportResponse,
  type ProjectListResponse,
  type ProjectWriteInput,
} from '@/api/schemas/project'
import { z } from 'zod'

export type ProjectListQuery = {
  q?: string | undefined
  category?: ProjectCategory | undefined
}

export function listProjects(query: ProjectListQuery): Promise<ProjectListResponse> {
  return request({
    method: 'GET',
    path: '/api/v1/projects',
    query,
    schema: ProjectListResponseSchema,
    headers: authHeaders(),
  })
}

export function getProject(slug: string): Promise<ProjectDetail> {
  return request({
    method: 'GET',
    path: `/api/v1/projects/${encodeURIComponent(slug)}`,
    schema: ProjectDetailSchema,
    headers: authHeaders(),
  })
}

export function createProject(input: ProjectWriteInput): Promise<ProjectDetail> {
  return request({
    method: 'POST',
    path: '/api/v1/projects',
    body: input,
    schema: ProjectDetailSchema,
    headers: authHeaders(),
  })
}

export function updateProject(currentSlug: string, input: ProjectWriteInput): Promise<ProjectDetail> {
  return request({
    method: 'PUT',
    path: `/api/v1/projects/${encodeURIComponent(currentSlug)}`,
    body: input,
    schema: ProjectDetailSchema,
    headers: authHeaders(),
  })
}

export function archiveProject(slug: string): Promise<null> {
  return request({
    method: 'DELETE',
    path: `/api/v1/projects/${encodeURIComponent(slug)}`,
    schema: z.null(),
    headers: authHeaders(),
  })
}

export function importProjects(items: ProjectWriteInput[]): Promise<ProjectImportResponse> {
  return request({
    method: 'POST',
    path: '/api/v1/projects/import',
    body: { items },
    schema: ProjectImportResponseSchema,
    headers: authHeaders(),
  })
}

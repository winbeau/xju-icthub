import { request } from '@/api/client'
import { authHeaders } from '@/api/endpoints/auth'
import {
  ProjectDetailSchema,
  ProjectListResponseSchema,
  type ProjectCategory,
  type ProjectDetail,
  type ProjectListResponse,
} from '@/api/schemas/project'

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

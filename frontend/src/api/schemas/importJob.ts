import { z } from 'zod'
import { ProjectCategorySchema } from '@/api/schemas/project'

export const ImportInputSchema = z.object({
  id: z.string(),
  inputKind: z.enum(['file', 'link', 'prompt']),
  provider: z.string(),
  displayName: z.string(),
  sourceRef: z.string().nullable(),
  mimeType: z.string().nullable(),
  sizeBytes: z.number().int().nullable(),
  status: z.string(),
})

export const ImportArtifactSchema = z.object({
  id: z.string(),
  relativePath: z.string(),
  artifactKind: z.enum([
    'code',
    'document',
    'presentation',
    'video',
    'image',
    'archive',
    'data',
    'other',
  ]),
  mimeType: z.string().nullable(),
  sizeBytes: z.number().int().nonnegative(),
  extractor: z.string(),
  metadata: z.record(z.unknown()),
  isCoverCandidate: z.boolean(),
})

export const ImportJobEventSchema = z.object({
  id: z.number().int(),
  eventType: z.string(),
  status: z.string(),
  stage: z.string(),
  progress: z.number().int().min(0).max(100),
  message: z.string().nullable(),
  createdAt: z.string(),
})

export const ImportAnalysisSchema = z.object({
  projectDraft: z.object({
    name: z.string(),
    slug: z.string(),
    summary: z.string(),
    primaryCategory: ProjectCategorySchema,
    suggestedTags: z.array(z.string()),
    ownerName: z.string().nullable().optional(),
    sourceName: z.string().nullable().optional(),
    highestAward: z.string().nullable(),
    status: z.string(),
  }),
  artifactSummary: z.array(
    z.object({
      kind: ImportArtifactSchema.shape.artifactKind,
      count: z.number().int().nonnegative(),
      totalBytes: z.number().int().nonnegative(),
    }),
  ),
  warnings: z.array(z.string()),
  agent: z.object({
    status: z.string(),
    mode: z.string(),
    message: z.string(),
  }),
  capabilities: z.object({
    zipUpload: z.string(),
    githubLink: z.string(),
    mixedFiles: z.string(),
    codexAgent: z.string(),
    githubPublish: z.string(),
  }),
})

export const ImportJobSchema = z.object({
  id: z.string(),
  status: z.string(),
  stage: z.string(),
  progress: z.number().int().min(0).max(100),
  sourceKind: z.string(),
  sourceName: z.string(),
  analysisEngine: z.string(),
  errorMessage: z.string().nullable(),
  createdAt: z.string(),
  updatedAt: z.string(),
  attemptCount: z.number().int().nonnegative(),
  startedAt: z.string().nullable(),
  completedAt: z.string().nullable(),
  inputs: z.array(ImportInputSchema),
  artifacts: z.array(ImportArtifactSchema),
  events: z.array(ImportJobEventSchema),
  result: ImportAnalysisSchema.nullable(),
})

export type ImportJob = z.infer<typeof ImportJobSchema>
export type ImportArtifact = z.infer<typeof ImportArtifactSchema>

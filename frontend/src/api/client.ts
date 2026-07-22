import type { ZodType } from 'zod'

const apiBase = import.meta.env['VITE_API_BASE'] as string | undefined
const useMock = import.meta.env.DEV && import.meta.env['VITE_USE_MOCK'] !== 'false'
const baseURL = apiBase ?? ''
const mockLatencyMs = 120

export type HttpMethod = 'GET' | 'POST' | 'PUT' | 'PATCH' | 'DELETE'

export type MockRequest = {
  method: HttpMethod
  path: string
  query: URLSearchParams
  body: unknown
  headers: Headers
}

export type MockHandler = (request: MockRequest) => Promise<unknown> | unknown

type MockPattern = {
  method: HttpMethod
  regex: RegExp
  handler: MockHandler
}

const mockHandlers = new Map<string, MockHandler>()
const mockPatterns: MockPattern[] = []

export function registerMock(method: HttpMethod, path: string, handler: MockHandler): void {
  if (path.includes(':')) {
    const regex = new RegExp(`^${path.replace(/:[A-Za-z_]+/g, '[^/]+')}$`)
    mockPatterns.push({ method, regex, handler })
    return
  }
  mockHandlers.set(`${method} ${path}`, handler)
}

function findMockHandler(method: HttpMethod, path: string): MockHandler | undefined {
  return (
    mockHandlers.get(`${method} ${path}`) ??
    mockPatterns.find((pattern) => pattern.method === method && pattern.regex.test(path))?.handler
  )
}

export class ApiError extends Error {
  override readonly name = 'ApiError'

  constructor(
    message: string,
    public readonly status: number,
    public readonly path: string,
  ) {
    super(message)
  }
}

export type QueryValue =
  | string
  | number
  | boolean
  | ReadonlyArray<string | number | boolean>
  | undefined

type RequestOptions<T> = {
  method: HttpMethod
  path: string
  schema: ZodType<T>
  body?: unknown
  query?: Record<string, QueryValue>
  headers?: Record<string, string>
}

export async function request<T>(options: RequestOptions<T>): Promise<T> {
  const query = new URLSearchParams()
  for (const [key, value] of Object.entries(options.query ?? {})) {
    if (value === undefined) continue
    if (Array.isArray(value)) {
      value.forEach((item) => query.append(key, String(item)))
    } else {
      query.set(key, String(value))
    }
  }

  let raw: unknown

  if (useMock) {
    const handler = findMockHandler(options.method, options.path)
    if (!handler) {
      throw new ApiError(
        `No mock handler registered for ${options.method} ${options.path}`,
        501,
        options.path,
      )
    }
    await new Promise((resolve) => setTimeout(resolve, mockLatencyMs))
    raw = await handler({
      method: options.method,
      path: options.path,
      query,
      body: options.body,
      headers: new Headers(options.headers ?? {}),
    })
  } else {
    const url = new URL(`${baseURL}${options.path}`, window.location.origin)
    url.search = query.toString()
    const response = await fetch(url, {
      method: options.method,
      headers: {
        'Content-Type': 'application/json',
        ...(options.headers ?? {}),
      },
      ...(options.body === undefined ? {} : { body: JSON.stringify(options.body) }),
    })

    if (!response.ok) {
      let message = `HTTP ${response.status}`
      try {
        const body = (await response.json()) as {
          detail?: unknown
          message?: unknown
          error?: { message?: unknown }
        }
        const candidate = body.detail ?? body.message ?? body.error?.message
        if (typeof candidate === 'string' && candidate.length > 0) message = candidate
      } catch {
        // Keep the HTTP fallback when the server returns no JSON body.
      }
      throw new ApiError(message, response.status, options.path)
    }

    raw = response.status === 204 ? null : await response.json()
  }

  return options.schema.parse(raw)
}

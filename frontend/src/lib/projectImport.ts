import {
  ProjectWriteInputSchema,
  type ProjectResourceInput,
  type ProjectWriteInput,
} from '@/api/schemas/project'

const HEADER_ALIASES = {
  slug: ['slug', '项目路径'],
  name: ['name', '项目名', '名称'],
  category: ['category', '类别', '项目类别'],
  summary: ['summary', '简介', '项目简介'],
  award: ['award', '曾获奖', '获奖'],
  status: ['status', '状态', '项目状态'],
  owner: ['owner', '负责人', '目前负责'],
  source: ['source', '来源', '来源者'],
  critique: ['critique', '锐评'],
  tags: ['tags', '标签'],
  github: ['github', 'gh', 'gh链接'],
  baidu: ['baidu', '百度网盘', '百度网盘链接'],
} as const

type HeaderKey = keyof typeof HEADER_ALIASES

export function parseProjectImport(text: string): ProjectWriteInput[] {
  const lines = text
    .replace(/^\uFEFF/, '')
    .split(/\r?\n/)
    .map((line) => line.trim())
    .filter(Boolean)
  if (lines.length < 2) throw new Error('请粘贴表头和至少一行项目数据')
  if (lines.length > 201) throw new Error('单次最多导入 200 个项目')

  const delimiter = lines[0]!.includes('\t') ? '\t' : ','
  const headers = parseDelimitedLine(lines[0]!, delimiter).map((header) =>
    header.trim().toLowerCase(),
  )
  const indexes = resolveHeaderIndexes(headers)

  return lines.slice(1).map((line, rowIndex) => {
    const cells = parseDelimitedLine(line, delimiter).map((cell) => cell.trim())
    const read = (key: HeaderKey): string => {
      const index = indexes[key]
      return index === undefined ? '' : (cells[index] ?? '')
    }
    const resources: ProjectResourceInput[] = []
    if (read('github')) {
      resources.push({ type: 'github', title: 'GitHub 仓库', url: read('github') })
    }
    if (read('baidu')) {
      resources.push({ type: 'baidu', title: '百度网盘', url: read('baidu') })
    }

    const raw = {
      slug: read('slug'),
      name: read('name'),
      primaryCategory: read('category'),
      summary: read('summary'),
      highestAward: emptyToNull(read('award')),
      status: read('status') || '研发中',
      ownerName: emptyToNull(read('owner')),
      sourceName: emptyToNull(read('source')),
      critique: read('critique'),
      tags: read('tags')
        .split(/[|、;；]/)
        .map((tag) => tag.trim())
        .filter(Boolean),
      resources,
    }
    const parsed = ProjectWriteInputSchema.safeParse(raw)
    if (!parsed.success) {
      const message = parsed.error.issues[0]?.message ?? '数据格式不正确'
      throw new Error(`第 ${rowIndex + 2} 行：${message}`)
    }
    return parsed.data
  })
}

function resolveHeaderIndexes(headers: string[]): Partial<Record<HeaderKey, number>> {
  const indexes: Partial<Record<HeaderKey, number>> = {}
  for (const [key, aliases] of Object.entries(HEADER_ALIASES) as [HeaderKey, readonly string[]][]) {
    const index = headers.findIndex((header) => aliases.includes(header))
    if (index >= 0) indexes[key] = index
  }
  for (const required of ['slug', 'name', 'category', 'summary'] as const) {
    if (indexes[required] === undefined) {
      throw new Error(`缺少必需列：${HEADER_ALIASES[required].join(' / ')}`)
    }
  }
  return indexes
}

function parseDelimitedLine(line: string, delimiter: string): string[] {
  const cells: string[] = []
  let cell = ''
  let quoted = false
  for (let index = 0; index < line.length; index += 1) {
    const char = line[index]!
    if (char === '"') {
      if (quoted && line[index + 1] === '"') {
        cell += '"'
        index += 1
      } else {
        quoted = !quoted
      }
    } else if (char === delimiter && !quoted) {
      cells.push(cell)
      cell = ''
    } else {
      cell += char
    }
  }
  cells.push(cell)
  return cells
}

function emptyToNull(value: string): string | null {
  return value || null
}

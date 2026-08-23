'use strict'

const crypto = require('node:crypto')
const fs = require('node:fs')
const path = require('node:path')

const SCHEMA = 'yilong.web-ai-response-shape.v1'
const MAX_INPUT_BYTES = 8 * 1024 * 1024
const MAX_FRAMES = 256
const MAX_DEPTH = 12
const MAX_OBJECT_FIELDS = 128
const MAX_ARRAY_ITEMS = 64
const MAX_TOTAL_NODES = 20_000
const MAX_UNIQUE_ARRAY_SHAPES = 8
const PROVIDERS = new Set(['chatgpt', 'google-ai-mode'])

const SENSITIVE_KEY = /(?:^|[_-])(?:authorization|auth|cookie|cookies|credential|credentials|password|passwd|secret|session|tokens?|access[_-]?token|refresh[_-]?token|id[_-]?token|api[_-]?key|csrf|xsrf|signature|signed|headers?|set[_-]?cookie|account|email|phone|user[_-]?id|owner|request[_-]?token|device[_-]?id|profile[_-]?id)(?:$|[_-])/i
const DYNAMIC_KEY = /(?:^[0-9]+$|[0-9]{5,}|[0-9a-f]{8}-[0-9a-f-]{20,}|^[A-Za-z0-9+/=_-]{33,}$|[@/?#\s])/i

function sha256(value) {
  return crypto.createHash('sha256').update(value).digest('hex')
}

function canonical(value) {
  if (Array.isArray(value)) return `[${value.map(canonical).join(',')}]`
  if (value && typeof value === 'object') {
    return `{${Object.keys(value).sort().map((key) => `${JSON.stringify(key)}:${canonical(value[key])}`).join(',')}}`
  }
  return JSON.stringify(value)
}

function sizeBucket(value) {
  if (value <= 0) return '0'
  if (value <= 16) return '1-16'
  if (value <= 64) return '17-64'
  if (value <= 256) return '65-256'
  if (value <= 1024) return '257-1024'
  return '1025+'
}

function byteBucket(value) {
  if (value <= 1024) return '0-1KiB'
  if (value <= 16 * 1024) return '1-16KiB'
  if (value <= 256 * 1024) return '16-256KiB'
  if (value <= 1024 * 1024) return '256KiB-1MiB'
  return '1-8MiB'
}

function classifyString(value) {
  if (/^https?:\/\//i.test(value)) return 'url'
  if (/^(?:data|blob):/i.test(value)) return 'resource'
  if (/^\d{4}-\d{2}-\d{2}(?:[T ]|$)/.test(value)) return 'timestamp'
  if (/^(?:[0-9a-f]{8}-[0-9a-f-]{20,}|[A-Za-z0-9_-]{24,})$/i.test(value)) return 'identifier'
  return 'text'
}

function isSensitiveKey(value) {
  const normalized = String(value || '').replace(/([a-z0-9])([A-Z])/g, '$1_$2').toLowerCase()
  return SENSITIVE_KEY.test(normalized)
}

function parseResearchFrames(source) {
  const text = String(source || '').replace(/^\uFEFF/, '')
  if (!text.trim()) throw new Error('研究响应为空。')

  const lines = text.split(/\r?\n/)
  const looksLikeSse = lines.some((line) => /^\s*(?:data|event|id|retry):/.test(line))
  if (looksLikeSse) {
    const frames = []
    let dataLines = []
    const flush = () => {
      if (!dataLines.length) return
      const data = dataLines.join('\n').trim()
      dataLines = []
      if (!data || data === '[DONE]') return
      try {
        frames.push(JSON.parse(data))
      } catch (_) {
        throw new Error('SSE data 帧不是有效 JSON。')
      }
    }
    for (const line of lines) {
      if (!line.trim()) {
        flush()
      } else if (/^\s*data:/.test(line)) {
        dataLines.push(line.replace(/^\s*data:\s?/, ''))
      }
    }
    flush()
    if (!frames.length) throw new Error('SSE 中没有可研究的 JSON data 帧。')
    return { format: 'sse', frames }
  }

  try {
    return { format: 'json', frames: [JSON.parse(text)] }
  } catch (_) {
    const nonEmpty = lines.map((line) => line.trim()).filter(Boolean)
    if (nonEmpty.length < 2) throw new Error('研究响应不是有效 JSON、NDJSON 或 SSE。')
    try {
      return { format: 'ndjson', frames: nonEmpty.map((line) => JSON.parse(line)) }
    } catch (_) {
      throw new Error('研究响应不是有效 JSON、NDJSON 或 SSE。')
    }
  }
}

function createState() {
  return {
    nodes: 0,
    sensitiveFieldsDropped: 0,
    dynamicFieldsDropped: 0,
    objectFieldsTruncated: 0,
    arrayItemsTruncated: 0,
    depthTruncated: 0,
    nodesTruncated: 0
  }
}

function shapeOf(value, state, depth) {
  if (state.nodes >= MAX_TOTAL_NODES) {
    state.nodesTruncated += 1
    return { type: 'truncated', reason: 'node_limit' }
  }
  state.nodes += 1
  if (depth > MAX_DEPTH) {
    state.depthTruncated += 1
    return { type: 'truncated', reason: 'depth_limit' }
  }
  if (value === null) return { type: 'null' }
  if (Array.isArray(value)) {
    const candidates = value.slice(0, MAX_ARRAY_ITEMS).map((item) => shapeOf(item, state, depth + 1))
    if (value.length > MAX_ARRAY_ITEMS) state.arrayItemsTruncated += value.length - MAX_ARRAY_ITEMS
    const unique = new Map()
    for (const candidate of candidates) {
      const key = canonical(candidate)
      if (!unique.has(key) && unique.size < MAX_UNIQUE_ARRAY_SHAPES) unique.set(key, candidate)
    }
    if (new Set(candidates.map(canonical)).size > MAX_UNIQUE_ARRAY_SHAPES) state.arrayItemsTruncated += 1
    return { type: 'array', lengthBucket: sizeBucket(value.length), items: [...unique.values()] }
  }
  if (typeof value === 'object') {
    const keys = Object.keys(value).sort()
    const fields = []
    for (const key of keys.slice(0, MAX_OBJECT_FIELDS)) {
      if (isSensitiveKey(key)) {
        state.sensitiveFieldsDropped += 1
        continue
      }
      if (!/^[A-Za-z_][A-Za-z0-9_.:-]{0,63}$/.test(key) || DYNAMIC_KEY.test(key)) {
        state.dynamicFieldsDropped += 1
        continue
      }
      fields.push({ name: key, shape: shapeOf(value[key], state, depth + 1) })
    }
    if (keys.length > MAX_OBJECT_FIELDS) state.objectFieldsTruncated += keys.length - MAX_OBJECT_FIELDS
    return { type: 'object', fields }
  }
  if (typeof value === 'string') {
    return { type: 'string', class: classifyString(value), lengthBucket: sizeBucket([...value].length) }
  }
  if (typeof value === 'number') return { type: Number.isFinite(value) ? 'number' : 'non_finite_number' }
  if (typeof value === 'boolean') return { type: 'boolean' }
  return { type: 'unsupported' }
}

function sanitizeResearchResponse(source, providerId) {
  if (!PROVIDERS.has(providerId)) throw new Error('provider 必须是 chatgpt 或 google-ai-mode。')
  const input = Buffer.isBuffer(source) ? source : Buffer.from(String(source || ''), 'utf8')
  if (!input.length || input.length > MAX_INPUT_BYTES) throw new Error('研究响应大小必须在 1 B 到 8 MiB 之间。')
  const parsed = parseResearchFrames(input.toString('utf8'))
  const state = createState()
  const frameShapes = parsed.frames.slice(0, MAX_FRAMES).map((frame) => shapeOf(frame, state, 0))
  if (parsed.frames.length > MAX_FRAMES) state.arrayItemsTruncated += parsed.frames.length - MAX_FRAMES
  const uniqueFrames = new Map()
  for (const frame of frameShapes) {
    const key = canonical(frame)
    if (!uniqueFrames.has(key)) uniqueFrames.set(key, frame)
  }
  const shapes = [...uniqueFrames.values()]
  const shapeFingerprint = sha256(canonical(shapes))
  const truncationCount = state.objectFieldsTruncated + state.arrayItemsTruncated + state.depthTruncated + state.nodesTruncated
  return {
    schema: SCHEMA,
    providerId,
    sourceFormat: parsed.format,
    input: {
      byteLengthBucket: byteBucket(input.length),
      frameCountBucket: sizeBucket(parsed.frames.length),
      sha256: sha256(input)
    },
    structure: {
      sha256: shapeFingerprint,
      uniqueFrameShapes: shapes
    },
    sanitization: {
      policy: 'shape-only-no-values-v1',
      sensitiveFieldsDropped: state.sensitiveFieldsDropped,
      dynamicFieldsDropped: state.dynamicFieldsDropped,
      truncated: truncationCount > 0,
      truncationCount,
      totalNodesVisited: state.nodes
    }
  }
}

function parseArgs(argv) {
  const args = {}
  for (let index = 0; index < argv.length; index += 1) {
    const current = argv[index]
    if (!['--input', '--output', '--provider'].includes(current) || !argv[index + 1]) {
      throw new Error('用法：node scripts/sanitize-web-ai-response-fixture.cjs --input <本机文件> --output <脱敏.json> --provider <chatgpt|google-ai-mode>')
    }
    args[current.slice(2)] = argv[index + 1]
    index += 1
  }
  if (!args.input || !args.output || !args.provider) throw new Error('必须同时提供 --input、--output 和 --provider。')
  return args
}

function runCli() {
  const args = parseArgs(process.argv.slice(2))
  const inputPath = path.resolve(args.input)
  const outputPath = path.resolve(args.output)
  if (inputPath === outputPath) throw new Error('输出文件不能覆盖原始研究响应。')
  if (path.extname(outputPath).toLowerCase() !== '.json') throw new Error('脱敏输出必须使用 .json 扩展名。')
  const inputStat = fs.statSync(inputPath)
  if (!inputStat.isFile() || inputStat.size <= 0 || inputStat.size > MAX_INPUT_BYTES) {
    throw new Error('研究响应文件大小必须在 1 B 到 8 MiB 之间。')
  }
  const result = sanitizeResearchResponse(fs.readFileSync(inputPath), args.provider)
  fs.writeFileSync(outputPath, `${JSON.stringify(result, null, 2)}\n`, { encoding: 'utf8', flag: 'wx' })
  process.stdout.write(`WEB_AI_RESPONSE_FIXTURE_SANITIZED=1 schema=${SCHEMA} provider=${result.providerId} structure_sha256=${result.structure.sha256}\n`)
}

module.exports = Object.freeze({
  SCHEMA,
  parseResearchFrames,
  sanitizeResearchResponse
})

if (require.main === module) {
  try {
    runCli()
  } catch (error) {
    process.stderr.write(`WEB_AI_RESPONSE_FIXTURE_SANITIZED=0 ${error instanceof Error ? error.message : '未知错误。'}\n`)
    process.exitCode = 1
  }
}

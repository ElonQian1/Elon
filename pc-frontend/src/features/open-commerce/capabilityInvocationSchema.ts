export type CapabilitySchemaType =
  | 'object'
  | 'array'
  | 'string'
  | 'integer'
  | 'number'
  | 'boolean'
  | 'null'

export interface CapabilitySchemaNode {
  type?: CapabilitySchemaType
  title?: string
  description?: string
  default?: unknown
  enum?: unknown[]
  const?: unknown
  properties?: Record<string, CapabilitySchemaNode>
  required?: string[]
  additionalProperties?: boolean
  items?: CapabilitySchemaNode
  minProperties?: number
  maxProperties?: number
  minItems?: number
  maxItems?: number
  minLength?: number
  maxLength?: number
  minimum?: number
  maximum?: number
  exclusiveMinimum?: number
  exclusiveMaximum?: number
  format?: 'uuid' | 'date-time' | 'uri'
}

export interface CapabilityInputIssue {
  path: string
  message: string
}

export interface CapabilityInputBuildResult {
  input?: Record<string, unknown>
  issues: CapabilityInputIssue[]
}

interface NodeBuildResult {
  present: boolean
  value?: unknown
  issues: CapabilityInputIssue[]
}

const SUPPORTED_TYPES = new Set<CapabilitySchemaType>([
  'object',
  'array',
  'string',
  'integer',
  'number',
  'boolean',
  'null',
])
const MAX_FORM_DEPTH = 12
export const MAX_CAPABILITY_FORM_ITEMS = 50

export function asCapabilitySchema(value: Record<string, unknown>): CapabilitySchemaNode {
  return value as CapabilitySchemaNode
}

export function capabilitySchemaSupportIssue(value: Record<string, unknown>): string | null {
  if (!isRecord(value)) return '能力输入契约不是可识别的对象。'
  return inspectNode(value as CapabilitySchemaNode, '$', 0, true)
}

export function createCapabilityFormValue(value: Record<string, unknown>): unknown {
  return createNodeValue(asCapabilitySchema(value), true)
}

export function createCapabilityFieldValue(schema: CapabilitySchemaNode): unknown {
  return createNodeValue(schema, true)
}

export function buildCapabilityInput(
  schemaValue: Record<string, unknown>,
  formValue: unknown,
): CapabilityInputBuildResult {
  const supportIssue = capabilitySchemaSupportIssue(schemaValue)
  if (supportIssue) return { issues: [{ path: '$', message: supportIssue }] }
  const schema = asCapabilitySchema(schemaValue)
  const built = buildNode(schema, formValue, '$', true, 0)
  if (built.issues.length > 0) return { issues: built.issues }
  if (!isRecord(built.value)) {
    return { issues: [{ path: '$', message: '能力输入必须是对象。' }] }
  }
  return { input: built.value, issues: [] }
}

export function capabilityFieldLabel(name: string, schema: CapabilitySchemaNode): string {
  return schema.title?.trim() || name.replace(/_/g, ' ')
}

export function capabilitySchemaType(
  schema: CapabilitySchemaNode,
  root = false,
): CapabilitySchemaType {
  if (schema.type && SUPPORTED_TYPES.has(schema.type)) return schema.type
  if (schema.properties) return 'object'
  if (schema.items) return 'array'
  for (const candidate of [schema.const, schema.default, schema.enum?.[0]]) {
    const inferred = inferType(candidate)
    if (inferred) return inferred
  }
  return root ? 'object' : 'string'
}

function inspectNode(
  schemaValue: unknown,
  path: string,
  depth: number,
  root: boolean,
): string | null {
  if (!isRecord(schemaValue)) return `${path} 的字段契约无效。`
  const schema = schemaValue as unknown as CapabilitySchemaNode
  if (depth > MAX_FORM_DEPTH) return `表单嵌套超过 ${MAX_FORM_DEPTH} 层。`
  if (schema.type && !SUPPORTED_TYPES.has(schema.type)) return `${path} 使用了不支持的字段类型。`
  if (schema.enum !== undefined && !Array.isArray(schema.enum)) return `${path} 的选项列表无效。`
  if (schema.required !== undefined && (
    !Array.isArray(schema.required) || schema.required.some((name) => typeof name !== 'string')
  )) return `${path} 的必填字段列表无效。`
  const type = capabilitySchemaType(schema, root)
  if (root && type !== 'object') return '能力输入根节点必须是对象。'
  if (schema.enum?.some((item) => Array.isArray(item) || isRecord(item))) {
    return `${path} 使用了当前表单不支持的复杂枚举。`
  }
  if (type === 'array') {
    if (!schema.items) return `${path} 的列表没有声明项目结构。`
    if ((schema.minItems ?? 0) > MAX_CAPABILITY_FORM_ITEMS) {
      return `${path} 至少需要的项目数超过表单上限。`
    }
    const issue = inspectNode(schema.items, `${path}[]`, depth + 1, false)
    if (issue) return issue
  }
  if (type === 'object') {
    if (schema.properties !== undefined && !isRecord(schema.properties)) {
      return `${path} 的字段列表无效。`
    }
    const properties = schema.properties ?? {}
    for (const required of schema.required ?? []) {
      if (!hasOwn(properties, required)) {
        return `${path} 的必填字段 ${required} 没有可呈现的字段定义。`
      }
    }
    for (const [name, child] of Object.entries(properties)) {
      const issue = inspectNode(child, propertyPath(path, name), depth + 1, false)
      if (issue) return issue
    }
  }
  return null
}

function createNodeValue(schema: CapabilitySchemaNode, required: boolean): unknown {
  if (hasOwn(schema, 'const')) return required ? cloneJson(schema.const) : undefined
  if (hasOwn(schema, 'default')) return cloneJson(schema.default)
  if (schema.enum?.length === 1 && required) return cloneJson(schema.enum[0])
  if (!required) return undefined
  switch (capabilitySchemaType(schema)) {
    case 'object': {
      const requiredFields = new Set(schema.required ?? [])
      return Object.fromEntries(
        Object.entries(schema.properties ?? {}).map(([name, child]) => [
          name,
          createNodeValue(child, requiredFields.has(name)),
        ]),
      )
    }
    case 'array': {
      const count = Math.min(schema.minItems ?? 0, MAX_CAPABILITY_FORM_ITEMS)
      return Array.from({ length: count }, () => createNodeValue(schema.items ?? {}, true))
    }
    case 'boolean':
      return false
    case 'null':
      return null
    default:
      return ''
  }
}

function buildNode(
  schema: CapabilitySchemaNode,
  rawValue: unknown,
  path: string,
  required: boolean,
  depth: number,
): NodeBuildResult {
  if (depth > MAX_FORM_DEPTH) return issue(path, '字段嵌套过深。')
  if (hasOwn(schema, 'const')) {
    if (!required && rawValue === undefined) return absent()
    return { present: true, value: cloneJson(schema.const), issues: [] }
  }
  if (schema.enum) {
    if (rawValue === undefined) return required ? issue(path, '请选择一个选项。') : absent()
    if (!schema.enum.some((item) => sameJson(item, rawValue))) {
      return issue(path, '选择值不在商户声明的范围内。')
    }
  }
  switch (capabilitySchemaType(schema, path === '$')) {
    case 'object':
      return buildObject(schema, rawValue, path, required, depth)
    case 'array':
      return buildArray(schema, rawValue, path, required, depth)
    case 'string':
      return buildString(schema, rawValue, path, required)
    case 'integer':
    case 'number':
      return buildNumber(schema, rawValue, path, required)
    case 'boolean':
      if (rawValue === undefined) return required ? issue(path, '请选择开或关。') : absent()
      return typeof rawValue === 'boolean'
        ? { present: true, value: rawValue, issues: [] }
        : issue(path, '必须是开关值。')
    case 'null':
      return required || rawValue === null
        ? { present: true, value: null, issues: [] }
        : absent()
  }
}

function buildObject(
  schema: CapabilitySchemaNode,
  rawValue: unknown,
  path: string,
  required: boolean,
  depth: number,
): NodeBuildResult {
  if (!required && rawValue === undefined) return absent()
  const source = isRecord(rawValue) ? rawValue : {}
  const requiredFields = new Set(schema.required ?? [])
  const result: Record<string, unknown> = {}
  const issues: CapabilityInputIssue[] = []
  for (const [name, child] of Object.entries(schema.properties ?? {})) {
    const built = buildNode(
      child,
      source[name],
      propertyPath(path, name),
      requiredFields.has(name),
      depth + 1,
    )
    issues.push(...built.issues)
    if (built.present) result[name] = built.value
  }
  if (issues.length > 0) return { present: true, value: result, issues }
  const size = Object.keys(result).length
  if (!required && size === 0) return absent()
  if (schema.minProperties !== undefined && size < schema.minProperties) {
    return issue(path, `至少需要填写 ${schema.minProperties} 个字段。`)
  }
  if (schema.maxProperties !== undefined && size > schema.maxProperties) {
    return issue(path, `最多只能填写 ${schema.maxProperties} 个字段。`)
  }
  return { present: true, value: result, issues: [] }
}

function buildArray(
  schema: CapabilitySchemaNode,
  rawValue: unknown,
  path: string,
  required: boolean,
  depth: number,
): NodeBuildResult {
  const values = Array.isArray(rawValue) ? rawValue : []
  if (!required && values.length === 0) return absent()
  if (values.length > MAX_CAPABILITY_FORM_ITEMS) {
    return issue(path, `列表最多支持 ${MAX_CAPABILITY_FORM_ITEMS} 项。`)
  }
  if (schema.minItems !== undefined && values.length < schema.minItems) {
    return issue(path, `至少需要 ${schema.minItems} 项。`)
  }
  if (schema.maxItems !== undefined && values.length > schema.maxItems) {
    return issue(path, `最多只能填写 ${schema.maxItems} 项。`)
  }
  const results: unknown[] = []
  const issues: CapabilityInputIssue[] = []
  values.forEach((value, index) => {
    const built = buildNode(schema.items ?? {}, value, `${path}[${index + 1}]`, true, depth + 1)
    issues.push(...built.issues)
    if (built.present) results.push(built.value)
  })
  return { present: true, value: results, issues }
}

function buildString(
  schema: CapabilitySchemaNode,
  rawValue: unknown,
  path: string,
  required: boolean,
): NodeBuildResult {
  if (rawValue === undefined || rawValue === null) return required ? issue(path, '请填写此字段。') : absent()
  if (typeof rawValue !== 'string') return issue(path, '必须是文本。')
  if (!required && rawValue === '') return absent()
  let value = rawValue
  if (schema.format === 'date-time' && !schema.enum) {
    const parsed = new Date(value)
    if (Number.isNaN(parsed.getTime())) return issue(path, '请选择有效的日期和时间。')
    value = parsed.toISOString()
  }
  if (schema.minLength !== undefined && value.length < schema.minLength) {
    return issue(path, `至少需要 ${schema.minLength} 个字符。`)
  }
  if (schema.maxLength !== undefined && value.length > schema.maxLength) {
    return issue(path, `最多只能填写 ${schema.maxLength} 个字符。`)
  }
  if (schema.format === 'uuid' && !UUID_PATTERN.test(value)) {
    return issue(path, '请输入有效的 UUID。')
  }
  if (schema.format === 'uri') {
    try {
      new URL(value)
    } catch {
      return issue(path, '请输入包含协议的有效网址。')
    }
  }
  return { present: true, value, issues: [] }
}

function buildNumber(
  schema: CapabilitySchemaNode,
  rawValue: unknown,
  path: string,
  required: boolean,
): NodeBuildResult {
  if (rawValue === undefined || rawValue === '') return required ? issue(path, '请填写数值。') : absent()
  const value = typeof rawValue === 'number' ? rawValue : Number(rawValue)
  if (!Number.isFinite(value)) return issue(path, '请输入有效数值。')
  if (capabilitySchemaType(schema) === 'integer' && !Number.isInteger(value)) {
    return issue(path, '请输入整数。')
  }
  if (schema.minimum !== undefined && value < schema.minimum) return issue(path, `不能小于 ${schema.minimum}。`)
  if (schema.maximum !== undefined && value > schema.maximum) return issue(path, `不能大于 ${schema.maximum}。`)
  if (schema.exclusiveMinimum !== undefined && value <= schema.exclusiveMinimum) {
    return issue(path, `必须大于 ${schema.exclusiveMinimum}。`)
  }
  if (schema.exclusiveMaximum !== undefined && value >= schema.exclusiveMaximum) {
    return issue(path, `必须小于 ${schema.exclusiveMaximum}。`)
  }
  return { present: true, value, issues: [] }
}

function inferType(value: unknown): CapabilitySchemaType | null {
  if (value === null) return 'null'
  if (Array.isArray(value)) return 'array'
  if (isRecord(value)) return 'object'
  if (typeof value === 'string') return 'string'
  if (typeof value === 'boolean') return 'boolean'
  if (typeof value === 'number') return Number.isInteger(value) ? 'integer' : 'number'
  return null
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null && !Array.isArray(value)
}

export function hasOwn(value: object, key: PropertyKey): boolean {
  return Object.prototype.hasOwnProperty.call(value, key)
}

function issue(path: string, message: string): NodeBuildResult {
  return { present: false, issues: [{ path, message }] }
}

function absent(): NodeBuildResult {
  return { present: false, issues: [] }
}

function cloneJson<T>(value: T): T {
  if (value === undefined) return value
  return JSON.parse(JSON.stringify(value)) as T
}

function sameJson(left: unknown, right: unknown): boolean {
  return JSON.stringify(left) === JSON.stringify(right)
}

function propertyPath(parent: string, name: string): string {
  return parent === '$' ? name : `${parent}.${name}`
}

const UUID_PATTERN = /^[0-9a-f]{8}-[0-9a-f]{4}-[1-8][0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/i

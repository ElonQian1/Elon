import type { CSSProperties } from 'react'
import { Plus, Trash2 } from 'lucide-react'
import {
  MAX_CAPABILITY_FORM_ITEMS,
  capabilityFieldLabel,
  capabilitySchemaType,
  createCapabilityFieldValue,
  hasOwn,
  type CapabilitySchemaNode,
} from './capabilityInvocationSchema'
import { actionStyle, commerceStyles } from './openCommerceStyles'

interface CapabilitySchemaFieldProps {
  name: string
  path: string
  schema: CapabilitySchemaNode
  value: unknown
  required: boolean
  onChange: (value: unknown) => void
  root?: boolean
}

export default function CapabilitySchemaField(props: CapabilitySchemaFieldProps) {
  const { schema } = props
  if (hasOwn(schema, 'const')) return <FixedField {...props} />
  if (schema.enum) return <EnumField {...props} />

  switch (capabilitySchemaType(schema, props.root)) {
    case 'object':
      return <ObjectField {...props} />
    case 'array':
      return <ArrayField {...props} />
    case 'boolean':
      return <BooleanField {...props} />
    case 'null':
      return <NullField {...props} />
    case 'integer':
    case 'number':
      return <NumberField {...props} />
    case 'string':
      return <StringField {...props} />
  }
}

function ObjectField({ name, path, schema, value, required, onChange, root }: CapabilitySchemaFieldProps) {
  const object = isRecord(value) ? value : {}
  const requiredFields = new Set(schema.required ?? [])
  const entries = Object.entries(schema.properties ?? {})
  const body = entries.length === 0
    ? <p style={emptyStyle}>该能力不需要额外填写信息。</p>
    : entries.map(([fieldName, child]) => (
      <CapabilitySchemaField
        key={fieldName}
        name={fieldName}
        path={path === '$' ? fieldName : `${path}.${fieldName}`}
        schema={child}
        value={object[fieldName]}
        required={requiredFields.has(fieldName)}
        onChange={(next) => onChange({ ...object, [fieldName]: next })}
      />
    ))

  if (root) return <div style={fieldListStyle}>{body}</div>
  return (
    <fieldset style={groupStyle}>
      <legend style={legendStyle}>{capabilityFieldLabel(name, schema)}{required && ' *'}</legend>
      {schema.description && <p style={descriptionStyle}>{schema.description}</p>}
      <div style={fieldListStyle}>{body}</div>
    </fieldset>
  )
}

function ArrayField({ name, path, schema, value, required, onChange }: CapabilitySchemaFieldProps) {
  const items = Array.isArray(value) ? value : []
  const maximum = Math.min(schema.maxItems ?? MAX_CAPABILITY_FORM_ITEMS, MAX_CAPABILITY_FORM_ITEMS)
  const minimum = schema.minItems ?? 0
  return (
    <fieldset style={groupStyle}>
      <legend style={legendStyle}>{capabilityFieldLabel(name, schema)}{required && ' *'}</legend>
      {schema.description && <p style={descriptionStyle}>{schema.description}</p>}
      <div style={fieldListStyle}>
        {items.map((item, index) => (
          <div key={`${path}-${index}`} style={arrayItemStyle}>
            <div style={commerceStyles.itemHeader}>
              <strong style={arrayTitleStyle}>第 {index + 1} 项</strong>
              <button
                type="button"
                style={actionStyle('icon', items.length <= minimum)}
                disabled={items.length <= minimum}
                title="删除此项"
                onClick={() => onChange(items.filter((_, itemIndex) => itemIndex !== index))}
              >
                <Trash2 size={13} />
              </button>
            </div>
            <CapabilitySchemaField
              name={`${name} ${index + 1}`}
              path={`${path}[${index + 1}]`}
              schema={schema.items ?? {}}
              value={item}
              required
              onChange={(next) => onChange(items.map((current, itemIndex) => (
                itemIndex === index ? next : current
              )))}
              root={capabilitySchemaType(schema.items ?? {}) === 'object'}
            />
          </div>
        ))}
        {items.length === 0 && <p style={emptyStyle}>尚未添加项目。</p>}
        <button
          type="button"
          style={actionStyle('secondary', items.length >= maximum)}
          disabled={items.length >= maximum}
          onClick={() => onChange([...items, createCapabilityFieldValue(schema.items ?? {})])}
        >
          <Plus size={13} />添加一项
        </button>
      </div>
    </fieldset>
  )
}

function EnumField({ name, schema, value, required, onChange }: CapabilitySchemaFieldProps) {
  const values = schema.enum ?? []
  const selectedIndex = values.findIndex((item) => JSON.stringify(item) === JSON.stringify(value))
  return (
    <label style={fieldStyle}>
      <FieldLabel name={name} schema={schema} required={required} />
      <select
        value={selectedIndex < 0 ? '' : String(selectedIndex)}
        onChange={(event) => onChange(event.target.value === '' ? undefined : values[Number(event.target.value)])}
      >
        <option value="">请选择</option>
        {values.map((item, index) => (
          <option key={`${index}:${JSON.stringify(item)}`} value={index}>{displayValue(item)}</option>
        ))}
      </select>
      <Description schema={schema} />
    </label>
  )
}

function StringField({ name, schema, value, required, onChange }: CapabilitySchemaFieldProps) {
  const inputType = schema.format === 'date-time' ? 'datetime-local' : schema.format === 'uri' ? 'url' : 'text'
  const text = typeof value === 'string' ? value : ''
  return (
    <label style={fieldStyle}>
      <FieldLabel name={name} schema={schema} required={required} />
      <input
        type={inputType}
        value={schema.format === 'date-time' ? dateTimeLocalValue(text) : text}
        minLength={schema.minLength}
        maxLength={schema.maxLength}
        required={required && (schema.minLength ?? 0) > 0}
        onChange={(event) => onChange(event.target.value)}
      />
      <Description schema={schema} />
    </label>
  )
}

function NumberField({ name, schema, value, required, onChange }: CapabilitySchemaFieldProps) {
  const type = capabilitySchemaType(schema)
  return (
    <label style={fieldStyle}>
      <FieldLabel name={name} schema={schema} required={required} />
      <input
        type="number"
        value={typeof value === 'number' || typeof value === 'string' ? value : ''}
        step={type === 'integer' ? 1 : 'any'}
        min={schema.minimum}
        max={schema.maximum}
        required={required}
        onChange={(event) => onChange(event.target.value)}
      />
      <Description schema={schema} />
    </label>
  )
}

function BooleanField({ name, schema, value, required, onChange }: CapabilitySchemaFieldProps) {
  if (!required) {
    return (
      <label style={fieldStyle}>
        <FieldLabel name={name} schema={schema} required={required} />
        <select
          value={value === undefined ? '' : value === true ? 'true' : 'false'}
          onChange={(event) => onChange(
            event.target.value === '' ? undefined : event.target.value === 'true',
          )}
        >
          <option value="">不提供</option>
          <option value="true">是</option>
          <option value="false">否</option>
        </select>
        <Description schema={schema} />
      </label>
    )
  }
  return (
    <label style={{ ...fieldStyle, ...commerceStyles.checkRow }}>
      <input type="checkbox" checked={value === true} onChange={(event) => onChange(event.target.checked)} />
      <span><FieldLabel name={name} schema={schema} required={required} /></span>
      <Description schema={schema} />
    </label>
  )
}

function NullField({ name, schema, value, required, onChange }: CapabilitySchemaFieldProps) {
  if (required) return <FixedField name={name} path="" schema={schema} value={null} required onChange={onChange} />
  return (
    <label style={fieldStyle}>
      <FieldLabel name={name} schema={schema} required={required} />
      <select
        value={value === null ? 'null' : ''}
        onChange={(event) => onChange(event.target.value === 'null' ? null : undefined)}
      >
        <option value="">不提供</option>
        <option value="null">提供空值</option>
      </select>
      <Description schema={schema} />
    </label>
  )
}

function FixedField({ name, schema, value, required, onChange }: CapabilitySchemaFieldProps) {
  const fixedValue = hasOwn(schema, 'const') ? schema.const : value
  if (!required && hasOwn(schema, 'const')) {
    return (
      <label style={fieldStyle}>
        <FieldLabel name={name} schema={schema} required={required} />
        <select
          value={value === undefined ? '' : 'fixed'}
          onChange={(event) => onChange(event.target.value === 'fixed' ? schema.const : undefined)}
        >
          <option value="">不提供</option>
          <option value="fixed">使用固定值：{displayValue(schema.const)}</option>
        </select>
        <Description schema={schema} />
      </label>
    )
  }
  return (
    <div style={fieldStyle}>
      <FieldLabel name={name} schema={schema} required={required} />
      <code style={fixedStyle}>{displayValue(fixedValue)}</code>
      <Description schema={schema} />
    </div>
  )
}

function FieldLabel({ name, schema, required }: Pick<CapabilitySchemaFieldProps, 'name' | 'schema' | 'required'>) {
  return <strong style={labelStyle}>{capabilityFieldLabel(name, schema)}{required ? ' *' : '（选填）'}</strong>
}

function Description({ schema }: { schema: CapabilitySchemaNode }) {
  return schema.description ? <small style={descriptionStyle}>{schema.description}</small> : null
}

function displayValue(value: unknown): string {
  if (value === null) return '空值'
  if (typeof value === 'boolean') return value ? '是' : '否'
  if (typeof value === 'string') return value
  return JSON.stringify(value)
}

function dateTimeLocalValue(value: string): string {
  if (!value) return ''
  const parsed = new Date(value)
  if (Number.isNaN(parsed.getTime())) return value
  const local = new Date(parsed.getTime() - parsed.getTimezoneOffset() * 60_000)
  return local.toISOString().slice(0, 16)
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null && !Array.isArray(value)
}

const fieldStyle: CSSProperties = { display: 'grid', gap: 5, minWidth: 0 }
const fieldListStyle: CSSProperties = { display: 'grid', gap: 10 }
const groupStyle: CSSProperties = {
  minWidth: 0,
  margin: 0,
  padding: '4px 0 4px 12px',
  border: 0,
  borderLeft: '2px solid var(--line)',
}
const legendStyle: CSSProperties = { padding: '0 5px', color: 'var(--text)', fontSize: 11 }
const labelStyle: CSSProperties = { overflowWrap: 'anywhere', color: 'var(--text)', fontSize: 10 }
const descriptionStyle: CSSProperties = { margin: 0, color: 'var(--text-muted)', fontSize: 9 }
const emptyStyle: CSSProperties = { margin: 0, color: 'var(--text-muted)', fontSize: 10 }
const arrayItemStyle: CSSProperties = { display: 'grid', gap: 8, paddingTop: 8, borderTop: '1px solid var(--line)' }
const arrayTitleStyle: CSSProperties = { color: 'var(--text-muted)', fontSize: 10 }
const fixedStyle: CSSProperties = { overflowWrap: 'anywhere', color: '#a9ded2', fontSize: 10 }

const assert = require('node:assert/strict')
const fs = require('node:fs')
const path = require('node:path')
const ts = require('typescript')

const sourcePath = path.resolve(__dirname, '../src/features/open-commerce/capabilityInvocationSchema.ts')
const source = fs.readFileSync(sourcePath, 'utf8')
const compiled = ts.transpileModule(source, {
  compilerOptions: {
    module: ts.ModuleKind.CommonJS,
    target: ts.ScriptTarget.ES2022,
  },
}).outputText
const moduleRef = { exports: {} }
new Function('module', 'exports', 'require', compiled)(moduleRef, moduleRef.exports, require)

const {
  buildCapabilityInput,
  capabilitySchemaSupportIssue,
  createCapabilityFormValue,
} = moduleRef.exports

const orderSchema = {
  type: 'object',
  required: ['quote_id', 'delivery_mode', 'line_items', 'confirmed_by_user'],
  additionalProperties: false,
  properties: {
    quote_id: { type: 'string', format: 'uuid', title: '报价编号' },
    delivery_mode: { type: 'string', enum: ['pickup', 'delivery'], title: '履约方式' },
    line_items: {
      type: 'array',
      minItems: 1,
      maxItems: 10,
      items: {
        type: 'object',
        required: ['product_id', 'quantity'],
        additionalProperties: false,
        properties: {
          product_id: { type: 'string', minLength: 2 },
          quantity: { type: 'integer', minimum: 1, maximum: 20 },
          note: { type: 'string', maxLength: 60 },
        },
      },
    },
    requested_at: { type: 'string', format: 'date-time' },
    allow_substitution: { type: 'boolean' },
    optional_contact: {
      type: 'object',
      required: ['phone'],
      properties: { phone: { type: 'string', minLength: 1 } },
    },
    optional_constant: { const: 'consumer' },
    confirmed_by_user: { const: true },
  },
}

assert.equal(capabilitySchemaSupportIssue(orderSchema), null)
const orderValue = createCapabilityFormValue(orderSchema)
assert.equal(orderValue.confirmed_by_user, true)
assert.equal(orderValue.line_items.length, 1)
assert.equal(orderValue.allow_substitution, undefined)
assert.equal(orderValue.optional_contact, undefined)
assert.equal(orderValue.optional_constant, undefined)
orderValue.quote_id = '123e4567-e89b-12d3-a456-426614174000'
orderValue.delivery_mode = 'pickup'
orderValue.line_items[0].product_id = 'coffee-1'
orderValue.line_items[0].quantity = '2'
orderValue.line_items[0].note = ''
orderValue.requested_at = '2026-08-02T20:30'

const built = buildCapabilityInput(orderSchema, orderValue)
assert.deepEqual(built.issues, [])
assert.equal(built.input.line_items[0].quantity, 2)
assert.equal(built.input.line_items[0].note, undefined)
assert.equal(built.input.allow_substitution, undefined)
assert.equal(built.input.optional_contact, undefined)
assert.equal(built.input.optional_constant, undefined)
assert.equal(built.input.confirmed_by_user, true)
assert.equal(new Date(built.input.requested_at).toISOString(), built.input.requested_at)

const explicitFalse = structuredClone(orderValue)
explicitFalse.allow_substitution = false
assert.equal(buildCapabilityInput(orderSchema, explicitFalse).input.allow_substitution, false)

const explicitConstant = structuredClone(orderValue)
explicitConstant.optional_constant = 'consumer'
assert.equal(buildCapabilityInput(orderSchema, explicitConstant).input.optional_constant, 'consumer')

const invalidQuantity = structuredClone(orderValue)
invalidQuantity.line_items[0].quantity = '0'
const invalidQuantityResult = buildCapabilityInput(orderSchema, invalidQuantity)
assert.equal(invalidQuantityResult.input, undefined)
assert.ok(invalidQuantityResult.issues.some((issue) => (
  issue.path === 'line_items[1].quantity' && issue.message.includes('不能小于 1')
)))

const missingRequired = structuredClone(orderValue)
missingRequired.quote_id = ''
const missingRequiredResult = buildCapabilityInput(orderSchema, missingRequired)
assert.equal(missingRequiredResult.input, undefined)
assert.ok(missingRequiredResult.issues.some((issue) => (
  issue.path === 'quote_id' && issue.message.includes('UUID')
)))

const requiredPlainText = buildCapabilityInput({
  type: 'object',
  required: ['note'],
  properties: { note: { type: 'string' } },
}, { note: '' })
assert.deepEqual(requiredPlainText.issues, [])
assert.equal(requiredPlainText.input.note, '')

assert.match(
  capabilitySchemaSupportIssue({
    type: 'object',
    required: ['hidden_field'],
    properties: {},
  }),
  /没有可呈现的字段定义/,
)
assert.match(
  capabilitySchemaSupportIssue({
    type: 'object',
    properties: { mode: { enum: [{ secret: true }] } },
  }),
  /复杂枚举/,
)
assert.match(
  capabilitySchemaSupportIssue({
    type: 'object',
    properties: {
      items: { type: 'array', minItems: 51, items: { type: 'string' } },
    },
  }),
  /超过表单上限/,
)

console.log('Capability invocation schema tests passed')

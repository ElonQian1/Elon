'use strict'

const assert = require('node:assert/strict')
const fs = require('node:fs')
const os = require('node:os')
const path = require('node:path')
const { execFileSync } = require('node:child_process')

const root = path.resolve(__dirname, '..', '..')
const sanitizerPath = path.join(root, 'scripts', 'sanitize-web-ai-response-fixture.cjs')
const fixturePath = path.join(root, 'scripts', 'fixtures', 'web-ai-response-shape-synthetic.json')
const sanitizerSource = fs.readFileSync(sanitizerPath, 'utf8')
const fixture = JSON.parse(fs.readFileSync(fixturePath, 'utf8'))
const { SCHEMA, parseResearchFrames, sanitizeResearchResponse } = require(sanitizerPath)

assert.equal(SCHEMA, 'yilong.web-ai-response-shape.v1')
assert.doesNotMatch(sanitizerSource, /\b(?:fetch|XMLHttpRequest|WebSocket|https\.request|http\.request|playwright|puppeteer)\b/)

const chatgptRaw = JSON.stringify(fixture.chatgpt)
const chatgpt = sanitizeResearchResponse(chatgptRaw, 'chatgpt')
assert.equal(chatgpt.schema, SCHEMA)
assert.equal(chatgpt.providerId, 'chatgpt')
assert.equal(chatgpt.sourceFormat, 'json')
assert.ok(chatgpt.sanitization.sensitiveFieldsDropped >= 3)
assert.ok(chatgpt.structure.stableFieldPaths.length <= 96)

const chatgptOutput = JSON.stringify(chatgpt)
for (const forbidden of [
  'synthetic-secret-must-disappear',
  'synthetic-cookie-must-disappear',
  'synthetic-user-must-disappear',
  'Synthetic Bitcoin answer must disappear',
  'chatgpt.example.invalid',
  '77274',
  'authorization',
  'cookie',
  'user_id'
]) assert.equal(chatgptOutput.includes(forbidden), false, `sanitized output leaked ${forbidden}`)

const changedValues = structuredClone(fixture.chatgpt)
changedValues.message.content.title = 'Completely different text'
changedValues.message.content.price = 1
changedValues.message.content.source_url = 'https://different.example.invalid/other?secret=yes'
const changedValueResult = sanitizeResearchResponse(JSON.stringify(changedValues), 'chatgpt')
assert.equal(changedValueResult.structure.sha256, chatgpt.structure.sha256)
assert.notEqual(changedValueResult.input.sha256, chatgpt.input.sha256)

const changedStructure = structuredClone(changedValues)
changedStructure.message.content.chart.currency = 'USD'
const changedStructureResult = sanitizeResearchResponse(JSON.stringify(changedStructure), 'chatgpt')
assert.notEqual(changedStructureResult.structure.sha256, chatgpt.structure.sha256)

const sse = [
  'event: message',
  `data: ${JSON.stringify(fixture.google)}`,
  '',
  'data: [DONE]',
  ''
].join('\n')
const google = sanitizeResearchResponse(sse, 'google-ai-mode')
assert.equal(google.sourceFormat, 'sse')
assert.equal(google.structure.uniqueFrameShapes.length, 1)
const googleOutput = JSON.stringify(google)
for (const forbidden of [
  'synthetic-google-token-must-disappear',
  'Synthetic weather answer must disappear',
  'weather.example.invalid',
  'requestToken'
]) assert.equal(googleOutput.includes(forbidden), false, `sanitized output leaked ${forbidden}`)
assert.doesNotMatch(googleOutput, /"(?:temperature|value)"\s*:\s*23(?:\D|$)/)

const nestedGooglePayload = JSON.stringify({
  answerCard: {
    title: 'Synthetic nested answer must disappear',
    temperature: 31,
    requestToken: 'synthetic-nested-token-must-disappear'
  }
})
const googleBatched = [
  `)]}'`,
  '',
  String(nestedGooglePayload.length + 40),
  JSON.stringify([['wrb.fr', 'rpc-id', nestedGooglePayload, null, null, null, 'generic']])
].join('\n')
const parsedGoogleBatch = parseResearchFrames(googleBatched, 'google-ai-mode')
assert.equal(parsedGoogleBatch.format, 'google-batched-json')
const googleBatch = sanitizeResearchResponse(googleBatched, 'google-ai-mode')
assert.equal(googleBatch.sourceFormat, 'google-batched-json')
assert.equal(googleBatch.structure.protocol.family, 'google_batched_rpc')
assert.equal(googleBatch.structure.protocol.xssiPrefix, true)
assert.notEqual(googleBatch.structure.protocol.nestedJsonValueCountBucket, '0')
assert.notEqual(googleBatch.structure.protocol.rpcEnvelopeCountBucket, '0')
assert.ok(googleBatch.structure.stableFieldPaths.some((value) => value.endsWith('.answerCard')))
assert.ok(googleBatch.structure.stableFieldPaths.some((value) => value.endsWith('.answerCard.title')))
const googleBatchOutput = JSON.stringify(googleBatch)
for (const forbidden of [
  'Synthetic nested answer must disappear',
  'synthetic-nested-token-must-disappear',
  'rpc-id'
]) assert.equal(googleBatchOutput.includes(forbidden), false, `sanitized batch output leaked ${forbidden}`)
assert.doesNotMatch(googleBatchOutput, /"temperature"\s*:\s*31(?:\D|$)/)

const ndjson = `${JSON.stringify({ type: 'alpha', value: 1 })}\n${JSON.stringify({ type: 'beta', value: 2 })}\n`
assert.equal(parseResearchFrames(ndjson).format, 'ndjson')
assert.equal(sanitizeResearchResponse(ndjson, 'chatgpt').sourceFormat, 'ndjson')

const deep = {}
let cursor = deep
for (let index = 0; index < 20; index += 1) cursor = cursor.next = {}
assert.equal(sanitizeResearchResponse(JSON.stringify(deep), 'chatgpt').sanitization.truncated, true)
assert.throws(() => sanitizeResearchResponse('not json', 'chatgpt'), /不是有效 JSON/)
assert.throws(() => sanitizeResearchResponse('{}', 'unknown'), /provider/)

const tempDir = fs.mkdtempSync(path.join(os.tmpdir(), 'elon-web-ai-shape-'))
try {
  const outputPath = path.join(tempDir, 'sanitized.json')
  const output = execFileSync(process.execPath, [
    sanitizerPath,
    '--input', fixturePath,
    '--output', outputPath,
    '--provider', 'chatgpt'
  ], { encoding: 'utf8' })
  assert.match(output, /WEB_AI_RESPONSE_FIXTURE_SANITIZED=1/)
  const cliResult = JSON.parse(fs.readFileSync(outputPath, 'utf8'))
  assert.equal(cliResult.schema, SCHEMA)
  assert.equal(JSON.stringify(cliResult).includes('synthetic-secret-must-disappear'), false)
  assert.throws(() => execFileSync(process.execPath, [
    sanitizerPath,
    '--input', fixturePath,
    '--output', outputPath,
    '--provider', 'chatgpt'
  ], { encoding: 'utf8', stdio: 'pipe' }))
} finally {
  fs.rmSync(tempDir, { recursive: true, force: true })
}

console.log('web ai response fixture sanitizer contract passed')

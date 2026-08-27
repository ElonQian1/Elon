const assert = require('node:assert/strict')
const fs = require('node:fs')
const Module = require('node:module')
const path = require('node:path')
const ts = require('typescript')

const filename = path.resolve(__dirname, '../src/features/user-browser/localAiResponseTracking.ts')
const output = ts.transpileModule(fs.readFileSync(filename, 'utf8'), {
  compilerOptions: { module: ts.ModuleKind.CommonJS, target: ts.ScriptTarget.ES2020 },
  fileName: filename,
}).outputText
const compiled = new Module(filename, module)
compiled.filename = filename
compiled.paths = module.paths
compiled._compile(output, filename)

const {
  lastMatchingLocalAiUserIndex,
  latestLocalAiAssistantForUserTurn,
  matchingLocalAiUserCount,
  matchingLocalAiUserIndex,
  normalizeLocalAiResponsePrompt,
} = compiled.exports
const messages = [
  { role: 'user', content: [{ type: 'text', text: '重复  问题' }] },
  { role: 'assistant', content: [{ type: 'text', text: '旧回答' }] },
  { role: 'user', content: [{ type: 'markdown', text: '重复 问题' }] },
]

assert.equal(normalizeLocalAiResponsePrompt('  重复\n问题 '), '重复 问题')
assert.equal(lastMatchingLocalAiUserIndex(messages, '重复 问题'), 2)
assert.equal(lastMatchingLocalAiUserIndex(messages, '不存在'), -1)
assert.equal(matchingLocalAiUserCount(messages, '重复 问题'), 2)
assert.equal(matchingLocalAiUserIndex(messages, '重复 问题', 0), 0)
assert.equal(matchingLocalAiUserIndex(messages, '重复 问题', 1), 2)
assert.equal(matchingLocalAiUserIndex(messages, '重复 问题', 2), -1)
assert.equal(matchingLocalAiUserIndex(messages, '重复 问题', Number.NaN), 0)

const staged = [
  { id: 'u1', role: 'user', content: [] },
  { id: 'a-progress', role: 'assistant', content: [] },
  { id: 'a-final', role: 'assistant', content: [] },
  { id: 'u2', role: 'user', content: [] },
  { id: 'a-next', role: 'assistant', content: [] },
]
assert.equal(latestLocalAiAssistantForUserTurn(staged, 0)?.id, 'a-final')
assert.equal(latestLocalAiAssistantForUserTurn(staged, 3)?.id, 'a-next')
assert.equal(latestLocalAiAssistantForUserTurn(staged, 1), undefined)

process.stdout.write('PASS local AI response prompt tracking\n')

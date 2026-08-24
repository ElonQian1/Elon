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

const { lastMatchingLocalAiUserIndex, normalizeLocalAiResponsePrompt } = compiled.exports
const messages = [
  { role: 'user', content: [{ type: 'text', text: '重复  问题' }] },
  { role: 'assistant', content: [{ type: 'text', text: '旧回答' }] },
  { role: 'user', content: [{ type: 'markdown', text: '重复 问题' }] },
]

assert.equal(normalizeLocalAiResponsePrompt('  重复\n问题 '), '重复 问题')
assert.equal(lastMatchingLocalAiUserIndex(messages, '重复 问题'), 2)
assert.equal(lastMatchingLocalAiUserIndex(messages, '不存在'), -1)

process.stdout.write('PASS local AI response prompt tracking\n')

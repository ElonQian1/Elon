const assert = require('node:assert/strict')
const fs = require('node:fs')
const Module = require('node:module')
const path = require('node:path')
const ts = require('typescript')

const filename = path.resolve(
  __dirname,
  '../src/features/user-browser/localAiCapabilityVerificationPolicy.ts',
)
const output = ts.transpileModule(fs.readFileSync(filename, 'utf8'), {
  compilerOptions: { module: ts.ModuleKind.CommonJS, target: ts.ScriptTarget.ES2020 },
  fileName: filename,
}).outputText
const compiled = new Module(filename, module)
compiled.filename = filename
compiled.paths = module.paths
compiled._compile(output, filename)
const { localAiCapabilityVerificationFallback } = compiled.exports

const presets = [{ id: 'chatgpt' }, { id: 'google-ai-mode' }]
assert.deepEqual(localAiCapabilityVerificationFallback(presets, false, '暂时超时。'), {
  state: 'ready',
  providers: presets,
  message: '已继续使用 Win 私有能力预设；后台运行时核对暂未完成。暂时超时。',
})
assert.deepEqual(localAiCapabilityVerificationFallback(presets, true, '需要升级。'), {
  state: 'upgrade_required',
  providers: [],
  message: '需要升级。',
})

process.stdout.write('PASS local AI preset-first capability verification fallback\n')

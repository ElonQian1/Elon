const assert = require('node:assert/strict')
const fs = require('node:fs')
const path = require('node:path')
const ts = require('typescript')

const root = path.resolve(__dirname, '..')
const read = (relativePath) => fs.readFileSync(path.join(root, relativePath), 'utf8')
const compile = (source, fileName) => ts.transpileModule(source, {
  compilerOptions: { module: ts.ModuleKind.CommonJS, target: ts.ScriptTarget.ES2022 },
  fileName,
}).outputText

function evaluate(source, fileName, dependencies = {}) {
  const module = { exports: {} }
  const localRequire = (id) => {
    if (Object.hasOwn(dependencies, id)) return dependencies[id]
    throw new Error(`Unexpected dependency in ${fileName}: ${id}`)
  }
  new Function('exports', 'module', 'require', compile(source, fileName))(
    module.exports,
    module,
    localRequire,
  )
  return module.exports
}

const richProtocol = evaluate(
  read('src/features/user-browser/richContentProtocol.ts'),
  'richContentProtocol.ts',
)
const compatibility = evaluate(
  read('src/features/user-browser/localAiRendererCompatibility.ts'),
  'localAiRendererCompatibility.ts',
  { './richContentProtocol': richProtocol },
)

assert.equal(compatibility.localAiRendererCompatibility([
  { type: 'code', text: 'sample', language: 'ts' },
  { type: 'table', text: 'rows', rowCount: 2, columnCount: 3 },
  { type: 'math', text: 'x + 1' },
]), undefined)
assert.deepEqual(compatibility.localAiRendererCompatibility([
  { type: 'chart', text: 'interactive chart' },
]), { reason: 'unsupported_rich_part' })
assert.deepEqual(compatibility.localAiRendererCompatibility([{
  type: 'rich_card',
  text: 'future card',
  richContent: { schema: 'vendor.future.v2', kind: 'finance', payload: {} },
}]), { reason: 'unsupported_schema' })

console.log('AI renderer compatibility tests passed')

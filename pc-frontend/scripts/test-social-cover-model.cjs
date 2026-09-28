const assert = require('node:assert/strict')
const fs = require('node:fs')
const vm = require('node:vm')
const ts = require('typescript')
const { test } = require('node:test')
const path = require('node:path')
const root = path.resolve(__dirname, '../..')
function load(relative, globals = {}) {
  const exports = {}
  const context = vm.createContext({ exports, require: () => ({}), URL, Date, console, ...globals })
  const source = fs.readFileSync(path.join(root, relative), 'utf8')
  vm.runInContext(ts.transpileModule(source, { compilerOptions: { module: ts.ModuleKind.CommonJS, target: ts.ScriptTarget.ES2022 } }).outputText, context)
  return { exports, context }
}
test('media title alone cannot finish read-back; a later cover completes it', () => {
  const adapter = { identity: () => 'douyin:12345678', readSource: url => url, validate: (_, value) => value }
  const { exports: model } = load('pc-frontend/src/features/friends/socialReadPreview.ts', { ElonSocialReadAdapter: adapter })
  const original = { url: 'https://www.douyin.com/video/12345678' }
  assert.equal(model.readBackComplete(original, { title: 'Title', image: null }), false)
  assert.equal(model.readBackComplete(original, { title: 'Title', image: 'https://p3.douyinpic.com/a.jpg' }), true)
  adapter.identity = () => 'bilibili:BV19eYH6NEsC'
  assert.equal(model.readBackComplete(original, { title: 'Title', image: null }), false)
  adapter.identity = () => 'xiaohongshu:0123456789abcdef01234567'
  assert.equal(model.readBackComplete(original, { title: 'Title', image: null }), false)
  adapter.identity = () => 'weixin:article'
  assert.equal(model.readBackComplete(original, { title: 'Title', image: null }), true)
})
test('failed local poster can be evicted without erasing another account cache', () => {
  let rows = JSON.stringify(['owner-a', 'owner-b'].map(scope => ({ scope, original: 'https://example.com/', saved: Date.now(), value: {} })))
  const { exports: model } = load('pc-frontend/src/features/friends/socialReadPreview.ts', {
    localStorage: { getItem: () => rows, setItem: (_, value) => { rows = value } },
  })
  model.forgetRead('owner-a', { url: 'https://example.com/' })
  assert.deepEqual(JSON.parse(rows).map(e => e.scope), ['owner-b'])
})

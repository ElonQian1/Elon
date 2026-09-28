const assert = require('node:assert/strict')
const fs = require('node:fs')
const vm = require('node:vm')
const ts = require('typescript')
const { test } = require('node:test')
const path = require('node:path')
const root = path.resolve(__dirname, '../..')
test('public poster command is permitted in both production card hosts, with no broader commands', () => {
  const native = path.join(root, 'desktop-shell/src-tauri')
  for (const name of ['main', 'group-ai-worker']) {
    const capability = JSON.parse(fs.readFileSync(path.join(native, `capabilities/${name}.json`), 'utf8'))
    assert.ok(capability.permissions.includes('public-media-preview'))
  }
  const permission = fs.readFileSync(path.join(native, 'permissions/public-media-preview.toml'), 'utf8')
  assert.match(permission, /commands.allow = \["get_bilibili_public_preview"\]/)
  assert.doesNotMatch(permission, /commands.deny|shell:|fs:|http:/)
  assert.match(fs.readFileSync(path.join(native, 'build.rs'), 'utf8'), /"get_bilibili_public_preview"/)
  assert.match(fs.readFileSync(path.join(native, 'src/main.rs'), 'utf8'), /bilibili_preview::get_bilibili_public_preview/)
  const command = fs.readFileSync(path.join(native, 'src/bilibili_preview.rs'), 'utf8')
  assert.match(command, /ensure_caller\(&webview\)\?/)
})
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
test('Win public metadata uses native BV-only calls; old shells and short links retain server resolution', async () => {
  const calls = []
  let item = { url: 'https://www.bilibili.com/video/BV19eYH6NEsC/?t=30', site: '哔哩哔哩', embed: { kind: 'bilibili', id: 'BV19eYH6NEsC' } }
  let oldShell = false
  const server = { ...item, title: 'Server', image: null }
  const { exports: model } = load('pc-frontend/src/features/friends/socialCardPreview.ts', {
    DOMException,
    ElonSocialLinks: { links: () => [item] },
    require: name => name.includes('desktopShell') ? { getDesktopInvoke: () => async (command, args) => {
      calls.push({ command, args }); if (oldShell) throw new Error('Unknown command')
      return { title: 'Native', author: 'Public', image: 'https://i0.hdslb.com/bfs/archive/poster.jpg' }
    } } : { previewApi: async () => { calls.push('server'); return server } },
  })
  const request = () => model.cardPreviewApi('/api/me/link-preview', { method: 'POST', body: JSON.stringify({ url: item.url }) })
  assert.equal((await request()).title, 'Native')
  assert.equal(calls.includes('server'), false)
  assert.deepEqual(JSON.parse(JSON.stringify(calls[0].args)), { bvid: 'BV19eYH6NEsC' })
  oldShell = true
  assert.equal((await request()).title, 'Server')
  oldShell = false; calls.length = 0
  item = { ...item, url: 'https://b23.tv/fixture', embed: null }; server.url = item.url
  assert.equal((await request()).title, 'Native')
  assert.equal(calls[0], 'server')
})

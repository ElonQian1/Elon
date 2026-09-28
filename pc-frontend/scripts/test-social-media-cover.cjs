const assert = require('node:assert/strict')
const fs = require('node:fs')
const path = require('node:path')
const vm = require('node:vm')
const ts = require('typescript')
const { test } = require('node:test')
function setup(invoke) {
  const result = {}, saved = [], remembered = []
  const code = fs.readFileSync(path.join(__dirname, '../src/features/friends/socialMediaCover.ts'), 'utf8')
  vm.runInNewContext(ts.transpileModule(code, { compilerOptions: { module: ts.ModuleKind.CommonJS } }).outputText, {
    exports: result, Date, Map, Promise,
    require: name => name.includes('desktopShell') ? { getDesktopInvoke: () => invoke } : {
      readSource: p => p.url, cachedRead: () => null,
      rememberRead: (s, p, v) => { saved.push(s); return { ...p, ...v } },
    },
    ElonSocialReadAdapter: { identity: u => u.includes('douyin') ? 'douyin:12345' : 'bilibili:123' },
    ElonSocialLinks: { remember: (s, p) => remembered.push([s, p]) },
  })
  return { ...result, saved, remembered }
}
const preview = id => ({ url: 'https://www.douyin.com/video/' + id, image: null })
test('single flight, serial queue, bounded backlog and cancelled queued cards', async () => {
  const releases = [], calls = []
  const m = setup((command, args) => new Promise(resolve => { calls.push([command, args]); releases.push(resolve) }))
  const a = m.readMediaCover('owner', preview(12345), () => true)
  assert.equal(m.readMediaCover('owner', preview(12345), () => true), a)
  let active = true
  const b = m.readMediaCover('owner', preview(12346), () => active)
  active = false
  assert.equal(calls.length, 1)
  releases.shift()({ image: 'cover' })
  await a; assert.equal(await b, null); assert.equal(calls.length, 1)
  const jobs = Array.from({ length: 10 }, (_, n) => m.readMediaCover('owner', preview(20000 + n), () => true))
  assert.equal(await jobs[9], null)
  for (let n = 0; n < 9; n++) { releases.shift()(null); await jobs[n] }
  assert.equal(calls.length, 10)
})
test('failure cooldown and existing covers avoid repeated browser loads', async () => {
  let calls = 0
  const m = setup(async () => { calls++; return null })
  await m.enrichMediaCover('owner', preview(12345), () => true)
  await m.enrichMediaCover('owner', preview(12345), () => true)
  await m.enrichMediaCover('owner', { ...preview(12346), image: 'cover' }, () => true)
  assert.equal(calls, 1)
})
test('account change never persists or broadcasts a completed read', async () => {
  let finish, active = true
  const m = setup(() => new Promise(resolve => { finish = resolve }))
  const work = m.enrichMediaCover('owner', preview(12345), () => active)
  active = false; finish({ image: 'cover' }); await work
  assert.equal(m.saved.length, 0); assert.equal(m.remembered.length, 0)
})
test('successful reads retain scope; capability registered end to end', async () => {
  const m = setup(async () => ({ image: 'cover' }))
  await m.enrichMediaCover('owner', preview(12345), () => true)
  assert.deepEqual(m.saved, ['owner']); assert.equal(m.remembered[0][0], 'owner')
  const root = path.join(__dirname, '../../desktop-shell/src-tauri')
  for (const name of ['src/main.rs', 'build.rs', 'permissions/public-media-preview.toml']) {
    assert.match(fs.readFileSync(path.join(root, name), 'utf8'), /get_social_media_read_preview/)
  }
  const shared = fs.readFileSync(path.join(__dirname, '../../server/src/assets/social_links.js'), 'utf8')
  assert.match(shared, /draw\(latest\); retry.disabled = false; options.enrichPreview\?\.\(latest\)/)
})

const assert = require('node:assert/strict')
const fs = require('node:fs')
const Module = require('node:module')
const path = require('node:path')
const ts = require('typescript')
function load(name) {
  const filename = path.resolve(__dirname, `../src/features/user-browser/${name}.ts`)
  const compiled = new Module(filename, module)
  compiled.filename = filename; compiled.paths = module.paths
  compiled._compile(ts.transpileModule(fs.readFileSync(filename, 'utf8'), {
    compilerOptions: { module: ts.ModuleKind.CommonJS, target:ts.ScriptTarget.ES2020 }, fileName:filename,
  }).outputText, filename)
  return compiled.exports
}
const { localAiDirectoryModel } = load('localAiDirectoryModel')
const { localAiDirectoryNeedsAutoSync } = load('localAiDirectoryAutoSync')
const row = (id, extra = {}) => ({ id, path:`/c/${id}`, title:id, active:false, ...extra })
const snapshot = { conversations: [row('pin', { pinned:true, pinnedAt:20 }), row('old', { updatedAt:1 }), row('new', { updatedAt:2 }),
  row('child', { projectId:'g-p-one' }), row('unknown', { groupLabel:'置顶' })],
  projects: [row('g-p-one', { path:'/g/g-p-one/project', pinned:true, pinnedAt:10 }), row('g-p-two')] }
const view = localAiDirectoryModel(snapshot)
assert.deepEqual(view.pinned.map(row => row.id), ['pin', 'g-p-one'])
assert.deepEqual(view.projects.map(row => row.id), ['g-p-two'])
assert.deepEqual(view.recent.map(row => row.id), ['new','old','unknown'])
assert.equal(view.children('g-p-one')[0].id, 'child')
assert.equal(localAiDirectoryModel(snapshot, 'child').recent[0].id, 'child')
assert.equal(localAiDirectoryNeedsAutoSync({ navigationEvent: { collection: { complete:false, refreshSettled:true } }, navigationUpdatedAtMs:1000, nowMs:2000 }), false)
assert.equal(localAiDirectoryNeedsAutoSync({ navigationEvent: { collection: { complete:false, refreshSettled:true } }, navigationUpdatedAtMs:1000, nowMs:122000 }), true)
assert.equal(localAiDirectoryNeedsAutoSync({ navigationEvent: { collection: { complete:false, refreshSettled:true, continueRefresh:true } }, navigationUpdatedAtMs:1000, nowMs:2000 }), true)
console.log('PASS native directory grouping, project membership, ordering and bounded refresh freshness')

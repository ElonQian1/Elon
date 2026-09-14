const assert = require('node:assert/strict')
const fs = require('node:fs')
const path = require('node:path')
const vm = require('node:vm')
const ts = require('../pc-frontend/node_modules/typescript')
const root = path.resolve(__dirname, '..')
const source = fs.readFileSync(path.join(root, 'pc-frontend/src/features/friends/groupMessageRevisions.ts'), 'utf8')
const target = { exports: {} }
vm.runInNewContext(ts.transpileModule(source, { compilerOptions: { module: ts.ModuleKind.CommonJS, target: ts.ScriptTarget.ES2020 } }).outputText, { exports: target.exports })
const web = {}
vm.runInNewContext(fs.readFileSync(path.join(root, 'server/src/assets/group_message_revisions.js'), 'utf8'), web)
for (const diff of [target.exports.changedText, web.ElonGroupMessageRevisions.changedText]) {
  for (const [a, b, removed, added] of [
    ['八点\n🐲见', '九点\n🐲见', '八', '九'], ['hi🐲', 'hi🙂', '🐲', '🙂'],
    ['同样文字', '同样文字', '', ''], ['', '新增', '', '新增'], ['删除', '', '删除', ''],
    ['🐲开始与结尾🙂', '🐲新的结尾🙂', '开始与', '新的'],
    ['<script>text</script>', '<script>safe</script>', 'text', 'safe'],
  ]) assert.deepEqual(JSON.parse(JSON.stringify(diff(a, b))), { removed, added })
}
const message = { id: 'm', sender_user_id: 'u', created_at: 'original-time', outgoing: true, content: 'first', revision: 1 }
const latest = { ...message, content: 'third', revision: 3, attachments: [{ url: '/original.png' }] }
const pending = { ...message, id: 'tmp-1', content: 'sending' }
const { mergeGroupMessages, revisionsPath } = target.exports
let merged = mergeGroupMessages([latest, pending], [{ ...message, revision: 2 }])
assert.equal(merged[0].content, 'third')
assert.equal(merged[0].created_at, 'original-time')
assert.equal(merged[0].attachments[0].url, '/original.png')
assert.equal(merged[1].id, 'tmp-1')
const recalled = { ...latest, recalled_at: 'recalled' }
assert.equal(mergeGroupMessages([latest], [recalled])[0].recalled_at, 'recalled')
assert.equal(mergeGroupMessages([recalled], [latest])[0].recalled_at, 'recalled')
assert.equal(revisionsPath('g /?', 'm/#'), '/api/me/groups/g%20%2F%3F/messages/m%2F%23')
console.log('PASS: PC/PWA Unicode differences, stale refresh, recalled-message visibility, pending sends and encoded IDs')

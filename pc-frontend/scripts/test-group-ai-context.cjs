const assert = require('node:assert/strict')
const fs = require('node:fs')
const Module = require('node:module')
const path = require('node:path')
const ts = require('typescript')
const { test } = require('node:test')
const filename = path.resolve(__dirname, '../src/features/friends/group-ai/groupAiContext.ts')
const compiled = new Module(filename, module)
compiled._compile(ts.transpileModule(fs.readFileSync(filename, 'utf8'), {
  compilerOptions: { module: ts.ModuleKind.CommonJS, target: ts.ScriptTarget.ES2020 }, fileName: filename,
}).outputText, filename)
const { continuationDraft, prepareGroupContinuation, groupContinuation, clearGroupContinuation, contextPath } = compiled.exports
test('context route escapes identifiers', () => assert.equal(contextPath('g/x', 'a?'), '/api/me/groups/g%2Fx/messages/a%3F'))
test('continuation preserves markdown and roles without auto sending', () => {
  const draft = continuationDraft({ group_id: 'g', document: { title: 'Selected', messages: [
    { role: 'user', content: 'A: question' }, { role: 'assistant', content: '**Answer**\n```js\n1\n```' },
  ] } })
  assert.ok(draft.includes('**Answer**'))
  assert.ok(draft.includes('assistant'))
  assert.throws(() => continuationDraft({ document: { messages: [] } }))
  assert.throws(() => continuationDraft({ document: { messages: [{ role: 'user', content: 'x'.repeat(30001) }] } }))
})
test('handoff is owner scoped and consumable', () => {
  const id = prepareGroupContinuation('owner', 'group', 'Selected', 'private text')
  assert.equal(groupContinuation(id, 'other'), null)
  assert.equal(groupContinuation(id, 'owner').group, 'group')
  assert.equal(groupContinuation('wrong', 'owner'), null)
  clearGroupContinuation(id)
  assert.equal(groupContinuation(id, 'owner'), null)
})
test('new handoff invalidates the prior one', () => {
  const old = prepareGroupContinuation('owner', 'a', 'A', 'a')
  const next = prepareGroupContinuation('owner', 'b', 'B', 'b')
  assert.equal(groupContinuation(old, 'owner'), null)
  assert.equal(groupContinuation(next, 'owner').group, 'b')
  clearGroupContinuation(next)
})

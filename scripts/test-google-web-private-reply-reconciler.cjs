const assert = require('node:assert/strict')
const fs = require('node:fs')
const path = require('node:path')
const vm = require('node:vm')

const root = path.resolve(__dirname, '..')
const source = fs.readFileSync(
  path.join(root, 'android/app/src/main/assets/google_web_private_reply_reconciler.js'),
  'utf8',
)
const pageAdapter = fs.readFileSync(
  path.join(root, 'android/app/src/main/kotlin/com/elon/app/googleweb/GoogleWebPageAdapter.kt'),
  'utf8',
)
const adapter = fs.readFileSync(
  path.join(root, 'android/app/src/main/assets/google_web_adapter.js'),
  'utf8',
)

function loadReconciler() {
  const window = {}
  window.window = window
  const sandbox = { window, module: { exports: {} }, Object, String, Array, Set, URL }
  vm.runInNewContext(source, sandbox, { filename: 'google_web_private_reply_reconciler.js' })
  return sandbox.module.exports
}

const message = (role, text, id = role + '-' + text) => ({
  id,
  role,
  state: 'completed',
  content: [{ type: 'text', text }],
})
const reply = (prompt, text, streaming = false) => ({ prompt, text, streaming })
const contents = (messages) => messages.map((value) => [value.role, value.content[0].text])

{
  const reconciler = loadReconciler()
  const messages = [message('user', 'first')]
  assert.equal(reconciler.apply(messages, reply('first', 'answer one')), true)
  assert.deepEqual(contents(messages), [['user', 'first'], ['assistant', 'answer one']])
}

{
  const reconciler = loadReconciler()
  const messages = [
    message('user', 'first'),
    message('assistant', 'answer one'),
    message('user', 'second'),
    message('assistant', 'answer one', 'stale-copy'),
  ]
  assert.equal(reconciler.apply(messages, reply('second', 'answer two', true)), true)
  assert.deepEqual(contents(messages), [
    ['user', 'first'],
    ['assistant', 'answer one'],
    ['user', 'second'],
    ['assistant', 'answer two'],
  ])
  assert.equal(messages[3].state, 'streaming')
  assert.match(messages[3].id, /^google-private-answer-/)
}

{
  const reconciler = loadReconciler()
  const messages = [
    message('user', 'first'),
    message('assistant', 'same answer'),
    message('user', 'second'),
    message('assistant', 'same answer', 'stale-or-legitimate'),
  ]
  assert.equal(reconciler.apply(messages, reply('second', 'same answer')), true)
  assert.equal(messages[3].id, 'google-private-answer-2')
}

{
  const reconciler = loadReconciler()
  const messages = [
    message('user', 'first'),
    message('assistant', 'answer one'),
    message('user', 'second'),
    message('assistant', 'answer two'),
  ]
  assert.equal(reconciler.apply(messages, reply('second', 'private candidate')), false)
  assert.deepEqual(contents(messages).at(-1), ['assistant', 'answer two'])
}

{
  const reconciler = loadReconciler()
  const messages = [
    message('user', 'repeat'),
    message('assistant', 'old'),
    message('user', 'repeat'),
  ]
  assert.equal(reconciler.apply(messages, reply('repeat', 'new')), true)
  assert.deepEqual(contents(messages).at(-1), ['assistant', 'new'])
}

{
  const reconciler = loadReconciler()
  const first = [message('user', 'first')]
  assert.equal(reconciler.apply(first, reply('first', 'answer one')), true)
  const clippedSecondTurn = [
    message('user', 'second'),
    message('assistant', 'answer one', 'clipped-stale-copy'),
  ]
  assert.equal(reconciler.apply(clippedSecondTurn, reply('second', 'answer two')), true)
  assert.deepEqual(contents(clippedSecondTurn), [
    ['user', 'second'],
    ['assistant', 'answer two'],
  ])
}

{
  const reconciler = loadReconciler()
  reconciler.observePrompt([
    message('user', 'first'),
    message('assistant', 'answer one with source decoration'),
  ], 'second')
  const clippedSecondTurn = [
    message('user', 'second'),
    message('assistant', 'answer one with source decoration', 'reflowed-stale-copy'),
  ]
  assert.equal(reconciler.apply(clippedSecondTurn, reply('second', 'answer two')), true)
  assert.deepEqual(contents(clippedSecondTurn), [
    ['user', 'second'],
    ['assistant', 'answer two'],
  ])
}

{
  const reconciler = loadReconciler()
  const clippedWithoutCurrentPrompt = [
    message('user', 'first'),
    message('assistant', 'answer one'),
  ]
  const conversationUrl = 'https://www.google.com/search?udm=50&q=first'
  reconciler.observePrompt(clippedWithoutCurrentPrompt, 'second', conversationUrl)
  assert.equal(
    reconciler.apply(
      clippedWithoutCurrentPrompt,
      reply('second', 'answer two'),
      conversationUrl,
    ),
    true,
  )
  assert.deepEqual(contents(clippedWithoutCurrentPrompt), [
    ['user', 'first'],
    ['assistant', 'answer one'],
    ['user', 'second'],
    ['assistant', 'answer two'],
  ])
  assert.match(clippedWithoutCurrentPrompt[2].id, /^google-private-prompt-/)
}

{
  const reconciler = loadReconciler()
  const sourceUrl = 'https://www.google.com/search?udm=50&q=first&csuir=thread-one'
  reconciler.observePrompt([message('user', 'first')], 'second', sourceUrl)
  const destinationMessages = [message('user', 'unrelated')]
  assert.equal(reconciler.apply(
    destinationMessages,
    reply('second', 'answer two'),
    'https://www.google.com/search?udm=50&q=other&csuir=thread-two',
  ), false)
  assert.deepEqual(contents(destinationMessages), [['user', 'unrelated']])
}

assert.equal(loadReconciler().apply([], reply('', 'answer')), false)
assert.doesNotMatch(source, /document\.cookie|authorization|request\.headers|init\.headers|init\.body/i)
assert.match(adapter, /privateReplyReconciler\.observePrompt\(baseline\.messages, prompt, location\.href\)/)
assert.match(adapter, /privateReplyReconciler\.apply\(extraction\.messages, privateReply, location\.href\)/)
assert.match(pageAdapter, /"google_web_private_reply_reconciler\.js"/)
console.log('GOOGLE_WEB_PRIVATE_REPLY_RECONCILER_TESTS=passed')

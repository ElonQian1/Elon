const assert = require('node:assert/strict')
const fs = require('node:fs')
const path = require('node:path')
const vm = require('node:vm')
const ts = require('typescript')

function load(name, dependencies = {}) {
  const filename = path.resolve(__dirname, '../src/features/friends', name)
  const exports = {}
  const output = ts.transpileModule(fs.readFileSync(filename, 'utf8'), {
    compilerOptions: { module: ts.ModuleKind.CommonJS, jsx: ts.JsxEmit.ReactJSX, target: ts.ScriptTarget.ES2020 },
  }).outputText
  // Deliberately no crypto: the production HTTP WebView lacks randomUUID.
  vm.runInNewContext(output, { exports, require: id => {
    if (!(id in dependencies)) throw Error(`Unexpected dependency: ${id}`)
    return dependencies[id]
  } }, { filename })
  return exports
}

const ids = load('socialLocalId.ts')
assert.equal(new Set(Array.from({ length: 10000 }, () => ids.socialLocalId())).size, 10000)

function harness({ prepareFails = false, requestFails = false, pending = false } = {}) {
  const states = []
  const result = { input: 'hello', messages: [], calls: 0, sent: 0, preventions: 0 }
  let release
  const gate = new Promise(resolve => { release = resolve })
  const component = load('SocialComposer.tsx', {
    react: {
      useRef: current => ({ current }), useEffect: () => {},
      useState: initial => {
        const cell = { value: initial }; states.push(cell)
        return [initial, next => { cell.value = typeof next === 'function' ? next(cell.value) : next }]
      },
    },
    'react/jsx-runtime': { jsx: (type, props) => ({ type, props }), jsxs: (type, props) => ({ type, props }) },
    './socialChatCache': { conversationId: c => `${c.kind}:${c.id}` },
    './socialLocalId': { socialLocalId: () => {
      if (prepareFails) { prepareFails = false; throw Error('prepare failed') }
      return ids.socialLocalId()
    } },
    './socialChatOperations': { quoteText: () => '', sendSocialMessage: async () => {
      result.calls++
      if (pending) await gate
      if (requestFails) { requestFails = false; throw Error('request failed') }
      return { message: { id: `server-${result.calls}`, content: 'hello' } }
    } },
    './useSocialAttachments': { useSocialAttachments: () => ({ byConversation: {}, remove: () => {} }) },
    './SocialDialog': {}, './SocialTools.module.css': { default: {} }, './FriendsPage.module.css': { default: {} },
  }).default
  const set = field => next => { result[field] = typeof next === 'function' ? next(result[field]) : next }
  const tree = component({ conversation: { kind: 'group', id: 'fixture' }, title: 'Fixture', me: { id: 'me' },
    input: result.input, setInput: set('input'), setMessages: set('messages'), onSent: () => result.sent++, quote: null })
  function findForm(node) {
    if (!node || typeof node !== 'object') return null
    if (node.type === 'form') return node
    return [node.props?.children].flat(Infinity).map(findForm).find(Boolean)
  }
  const form = findForm(tree)
  assert.ok(form)
  async function submit() {
    form.props.onSubmit({ preventDefault: () => result.preventions++ })
    await new Promise(resolve => setImmediate(resolve))
  }
  return { result, states, submit, release }
}

async function main() {
  const normal = harness()
  await normal.submit()
  assert.equal(normal.result.calls, 1)
  assert.equal(normal.result.input, '')
  assert.equal(normal.result.messages[0].id, 'server-1')
  assert.equal(normal.result.sent, 1)

  for (const failure of ['prepareFails', 'requestFails']) {
    const h = harness({ [failure]: true })
    await h.submit()
    assert.equal(h.result.input, 'hello', 'failed send preserves draft')
    assert.equal(h.result.messages.length, 0)
    assert.equal(h.states[1].value['group:fixture'], false, 'busy state released')
    assert.match(h.states[2].value['group:fixture'], /failed/, 'error displayed')
    await h.submit()
    assert.equal(h.result.sent, 1, 'same composer can retry after failure')
  }

  const h = harness({ pending: true })
  await h.submit()
  await h.submit()
  assert.equal(h.result.calls, 1, 'double click does not duplicate in-flight request')
  h.release()
  await new Promise(resolve => setImmediate(resolve))
  assert.equal(h.result.sent, 1)
  console.log('PASS: HTTP-compatible local IDs; send, preparation failure, request failure, retry, duplicate click')
}
main().catch(error => { console.error(error); process.exitCode = 1 })

'use strict'

const assert = require('node:assert/strict')
const fs = require('node:fs')
const path = require('node:path')
const vm = require('node:vm')

const root = path.resolve(__dirname, '..')
const source = fs.readFileSync(path.join(
  root,
  'desktop-shell/src-tauri/src/local_ai_browser/chatgpt_win_private_stream_binding.js',
), 'utf8')

function install(window) {
  window.window = window
  vm.runInNewContext(source, { window, Object, Number }, {
    filename: 'chatgpt_win_private_stream_binding.js',
  })
  return window.__elonWinChatGptPrivateStreamBindingLifecycle
}

let disposed = 0
const stalePolicy = { id: 'stale-policy' }
const currentPolicy = { id: 'current-policy' }
const staleTransport = {
  version: 10,
  mergeMessages() {},
  dispose() { disposed += 1 },
}
const upgraded = {
  __elonChatGptPrivateStreamPolicy: currentPolicy,
  __elonChatGptPrivateStreamTransport: staleTransport,
  __elonWinChatGptPrivateStreamBinding: {
    version: 1,
    policy: stalePolicy,
    transport: staleTransport,
  },
}
const lifecycle = install(upgraded)
assert.equal(disposed, 1)
assert.equal(upgraded.__elonChatGptPrivateStreamTransport, undefined)

const currentTransport = { version: 11, mergeMessages() {}, dispose() { disposed += 1 } }
upgraded.__elonChatGptPrivateStreamTransport = currentTransport
assert.equal(lifecycle.commit(upgraded), true)
assert.equal(upgraded.__elonWinChatGptPrivateStreamBinding.policy, currentPolicy)
assert.equal(upgraded.__elonWinChatGptPrivateStreamBinding.transport, currentTransport)

install(upgraded)
assert.equal(disposed, 1, 'a transport bound to the current policy must remain installed')
assert.equal(upgraded.__elonChatGptPrivateStreamTransport, currentTransport)

upgraded.__elonChatGptPrivateStreamPolicy = { id: 'replacement-policy' }
install(upgraded)
assert.equal(disposed, 2, 'adapter reconnect must detach a transport bound to the previous policy')
assert.equal(upgraded.__elonChatGptPrivateStreamTransport, undefined)

console.log('PASS: Win private stream binding replaces stale transports and preserves current policy bindings')

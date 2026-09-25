const assert = require('node:assert/strict')
const fs = require('node:fs')
const path = require('node:path')
const vm = require('node:vm')

const root = path.resolve(__dirname, '..')
const host = path.join(root, 'desktop-shell/src-tauri/src/local_ai_browser')
const rust = fs.readFileSync(path.join(host, 'chatgpt_adapter_bootstrap.rs'), 'utf8')
const manifest = rust.slice(rust.indexOf('const ADAPTER_ASSETS:'), rust.indexOf('pub(super) fn initialization_script'))
const read = relative => fs.readFileSync(path.resolve(host, relative), 'utf8')
const includes = source => [...source.matchAll(/include_str!\(\s*"([^"]+\.js)"\s*\)/g)].map(match => match[1])
const files = includes(manifest)
const constants = Object.fromEntries([...rust.matchAll(/const (\w+): &str\s*=\s*include_str!\(\s*"([^"]+)"\s*\);/g)]
  .map(([, name, file]) => [name, read(file)]))

// Render the production template and its per-asset Rust format arguments. The test
// must fail on unrecognized assembly syntax instead of silently omitting a branch.
function script(omit) {
  const source = file => path.basename(file) === omit ? '' : read(file)
  const attachment = includes(read('attachment_bootstrap.rs')).map(source).join('\n')
  const branches = new Map([...rust.matchAll(/(?:if|else if) \*name == "([^"]+)" \{([\s\S]*?)(?=\n            \} else)/g)]
    .map(([, name, body]) => [name, body]))
  const adapters = [...manifest.matchAll(/\(\s*"([^"]+\.js)",([\s\S]*?)\n    \),/g)].map(([, name, expression]) => {
    const shared = `window.__elonChatGptBootstrapStage = '${name}';\n` + includes(expression).map(source).join('\n')
    const branch = branches.get(name)
    if (!branch) return shared
    const match = /format!\(\s*("(?:[^"\\]|\\.)*"),([\s\S]*?)\)\s*$/.exec(branch.trim())
    assert.ok(match, 'Unrecognized Rust assembly branch: ' + name)
    const values = match[2].trim().replace(/,$/, '').split(/,\s*/).map(arg => {
      if (arg === 'shared') return shared
      if (arg === 'super::attachment_bootstrap::initialization_script()') return attachment
      assert.ok(Object.hasOwn(constants, arg), 'Unrecognized assembly argument: ' + arg)
      return constants[arg]
    })
    let index = 0
    const rendered = JSON.parse(match[1]).replace(/\{\{|\}\}|\{\}/g, token => token === '{{' ? '{' : token === '}}' ? '}' : values[index++])
    assert.equal(index, values.length)
    return rendered
  }).join('\n')
  let template = /r#"([\s\S]*?)"#/.exec(rust)[1]
  const replacements = { ALLOWED_ORIGIN: 'https://chatgpt.com', ADAPTER_VERSION: /ADAPTER_VERSION: u32 = (\d+)/.exec(rust)[1],
    PRIVATE_JSON_REQUEST: constants.PRIVATE_JSON_REQUEST, PRIVATE_FETCH_TAP: constants.PRIVATE_FETCH_TAP,
    PRIVATE_SOCKET_TAP: constants.PRIVATE_SOCKET_TAP, PRIVATE_CONVERSATION_DIRECTORY: constants.PRIVATE_CONVERSATION_DIRECTORY,
    CAPTURED_FINANCE_RECOVERY: constants.WIN_CAPTURED_FINANCE_RECOVERY,
    RESPONSE_RESEARCH_CAPTURE: constants.WIN_RESPONSE_RESEARCH_CAPTURE.replaceAll('__PROVIDER_ID__', 'chatgpt'),
    ADAPTER_ASSETS: adapters }
  for (const [key, value] of Object.entries(replacements)) template = template.replace(`__${key}__`, () => value)
  assert.ok(!/__ADAPTER_ASSETS__|__PRIVATE_JSON_REQUEST__/.test(template))
  return template
}

function browser() {
  const events = []
  class Node {}
  class Element extends Node {
    querySelector() { return null } querySelectorAll() { return [] }
    getAttribute() { return null } matches() { return false } closest() { return null }
    addEventListener() {} removeEventListener() {} appendChild() {} remove() {}
    getBoundingClientRect() { return { top: 0, left: 0, width: 0, height: 0 } }
    style = {}; dataset = {}; classList = { add() {}, remove() {}, toggle() {} }
  }
  const location = { origin: 'https://chatgpt.com', href: 'https://chatgpt.com/', pathname: '/', search: '' }
  const document = { documentElement: new Element(), body: new Element(), head: new Element(),
    readyState: 'complete', title: 'ChatGPT', hidden: false,
    querySelector: () => null, querySelectorAll: () => [], getElementById: () => null,
    createElement: () => new Element(), addEventListener() {}, removeEventListener() {} }
  const window = { location, document, navigator: {}, innerWidth: 1280, innerHeight: 800,
    __elonChatGptAdapterTargetVersion: 210, __elonChatGptDocumentToken: 'doc_test_assembly',
    elonChatGptNative: { postMessage: text => events.push(JSON.parse(text)) },
    setTimeout: () => 1, clearTimeout() {}, setInterval: () => 1, clearInterval() {},
    addEventListener() {}, removeEventListener() {},
    getComputedStyle: () => ({ display: 'block', visibility: 'visible' }),
    sessionStorage: { getItem: () => null, setItem() {}, removeItem() {} },
    localStorage: { getItem: () => null, setItem() {}, removeItem() {} },
    __TAURI_INTERNALS__: { invoke: (_command, { payload }) => { events.push(JSON.parse(payload)); return Promise.resolve() } },
    fetch: () => Promise.reject(new Error('No network in assembly test')),
  }
  window.top = window; window.self = window
  const sandbox = vm.createContext({ window, document, location, navigator: window.navigator, Node, Element, HTMLElement: Element,
    HTMLInputElement: Element, HTMLTextAreaElement: Element, HTMLButtonElement: Element,
    MutationObserver: class { observe() {} disconnect() {} },
    URL, URLSearchParams, Headers, AbortController, TextEncoder, TextDecoder,
    setTimeout: window.setTimeout, clearTimeout: window.clearTimeout,
    setInterval: window.setInterval, clearInterval: window.clearInterval, console })
  return { window, sandbox, events }
}
function assemble(omit) {
  const context = browser()
  vm.runInContext(script(omit), context.sandbox, { filename: 'win-initialization.js' })
  return context
}
const ready = assemble()
assert.equal(ready.window.__elonChatGptBootstrapStage, 'ready', JSON.stringify(ready.events))
assert.equal(typeof ready.window.__elonChatGptLayout?.emitSnapshot, 'function')
assert.equal(typeof ready.window.__elonChatGptBridge?.command, 'function')
assert.equal(typeof ready.window.__elonChatGptPrivateTextRuntimeSubmit?.submit, 'function')
assert.ok(ready.events.some(envelope => envelope.event?.type === 'adapter_ready'))
const snapshots = []
ready.window.__elonChatGptLayout.emitSnapshot(event => snapshots.push(event), true)
assert.ok(snapshots.length > 0)
for (const omitted of ['chatgpt_web_adapter_control_labels.js', 'chatgpt_web_adapter_dictation_actions.js']) {
  const failed = assemble(omitted)
  assert.notEqual(failed.window.__elonChatGptBootstrapStage, 'ready')
  assert.ok(failed.events.some(event => event.kind === 'adapter_bootstrap_failed'))
}
console.log(`PASS Win template with ${files.length} shared assets, native injections and attachment/runtime sender reaches ready; missing dependencies fail closed`)

const assert = require('node:assert/strict')
const fs = require('node:fs')
const path = require('node:path')
const vm = require('node:vm')

const root = path.resolve(__dirname, '..')
const source = fs.readFileSync(path.join(root, 'desktop-shell/src-tauri/src/local_ai_browser/chatgpt_adapter_bootstrap.rs'), 'utf8')
const files = [...source.matchAll(/include_str!\("\.\.\/\.\.\/\.\.\/\.\.\/android\/app\/src\/main\/assets\/([^"/]+)"\)/g)]
  .map(match => match[1])
// Execute the actual Win composer dependency list and its actual consumer. No synthetic dependency mocks.
const composerAssets = files.filter(name => /^chatgpt_web_adapter_(composer|model_label|action_target|attachment|dictation)/.test(name))
const location = { origin: 'https://chatgpt.com' }
const window = { location, setTimeout, clearTimeout }
const sandbox = vm.createContext({ window, location, document: {}, setTimeout, clearTimeout, URL, console })
for (const name of composerAssets) {
  vm.runInContext(fs.readFileSync(path.join(root, 'android/app/src/main/assets', name), 'utf8'), sandbox, { filename: name })
}
assert.equal(typeof window.__elonChatGptDictationActions?.create, 'function')
assert.equal(typeof window.__elonChatGptComposer?.capabilities, 'function')
assert.ok(composerAssets.indexOf('chatgpt_web_adapter_dictation_actions.js') < composerAssets.indexOf('chatgpt_web_adapter_composer.js'))
console.log('PASS actual Win composer dependencies initialize before the composer consumer')

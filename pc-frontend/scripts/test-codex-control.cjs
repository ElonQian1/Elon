const fs = require('fs')
const path = require('path')
const root = path.resolve(__dirname, '..')
const read = (relative) => fs.readFileSync(path.join(root, relative), 'utf8')

const app = read('src/App.tsx')
const shell = read('src/features/shell/Shell.tsx')
const bridge = read('src/features/codex-control/useCodexControlBridge.ts')
const api = read('src/features/codex-control/codexControlApi.ts')
const page = read('src/features/codex-control/CodexControlPage.tsx')

if (!app.includes('path="codex-control"')) throw new Error('Codex control route missing')
if (!shell.includes('useCodexControlBridge()')) throw new Error('global Codex control bridge missing')
if (!bridge.includes('isControlEventUrl') || !bridge.includes("url.path === '/api/codex-control/events'")) throw new Error('network recorder must avoid recursive event logging')
if (bridge.includes('response.text()') || bridge.includes('request.body')) throw new Error('network diagnostics must not read request/response bodies')
if (!bridge.includes('claimWinAction') || !bridge.includes('codex_execute_semantic_action') || !bridge.includes('postWinActionReceipt')) throw new Error('Tauri actions must atomically claim and return receipts')
if (!api.includes("'/api/codex-control'")) throw new Error('loopback Codex control API missing')
for (const source of ['frontend', 'rust', 'cli', 'network', 'tauri', 'control']) {
  if (!page.includes(`id: '${source}'`)) throw new Error(`timeline source missing: ${source}`)
}
console.log('codex-control contract checks passed')

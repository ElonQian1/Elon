const assert = require('node:assert/strict')
const fs = require('node:fs')
const Module = require('node:module')
const path = require('node:path')
const ts = require('typescript')

const root = path.resolve(__dirname, '..', '..')
const read = (relativePath) => fs.readFileSync(path.join(root, relativePath), 'utf8')

const page = read('pc-frontend/src/features/ai/AiChatPage.tsx')
const pageStyles = read('pc-frontend/src/features/ai/AiChatPage.module.css')
const policy = read('pc-frontend/src/features/user-browser/localAiAnswerSurfacePolicy.ts')
const surface = read('pc-frontend/src/features/user-browser/AiOfficialAnswerSurface.tsx')
const backend = read('pc-frontend/src/features/user-browser/useAiWebChatBackend.ts')
const api = read('pc-frontend/src/features/user-browser/internalBrowserApi.ts')
const fullPage = read('pc-frontend/src/features/user-browser/AiBrowserExperience.tsx')
const embedded = read('desktop-shell/src-tauri/src/local_ai_browser/embedded_view.rs')

assert.match(page, /<AiOfficialAnswerSurface web=\{web\} \/>/)
assert.match(pageStyles, /\.feed\s*\{[^}]*position:\s*relative;/s)

assert.match(policy, /semanticCacheStatus === 'cached'/)
assert.match(policy, /semanticCacheStatus !== 'live'/)
assert.match(policy, /browserSurface !== 'chat'/)
assert.match(policy, /contextReady !== true/)
assert.match(policy, /snapshot\.streaming/)
assert.match(policy, /message\.role === 'assistant' && message\.state === 'completed'/)

const policyFilename = path.join(root, 'pc-frontend/src/features/user-browser/localAiAnswerSurfacePolicy.ts')
const policyOutput = ts.transpileModule(policy, {
  compilerOptions: { module: ts.ModuleKind.CommonJS, target: ts.ScriptTarget.ES2020 },
  fileName: policyFilename,
}).outputText
const compiledPolicy = new Module(policyFilename, module)
compiledPolicy.filename = policyFilename
compiledPolicy.paths = module.paths
compiledPolicy._compile(policyOutput, policyFilename)
const { selectLocalAiAnswerRenderMode } = compiledPolicy.exports
const liveSnapshot = {
  streaming: false,
  messages: [{ id: 'answer-1', role: 'assistant', state: 'completed', content: [] }],
}
const liveSession = {
  semanticCacheStatus: 'live', contextReady: true, loading: false, lastError: '', windowStatus: 'ready',
}
assert.equal(selectLocalAiAnswerRenderMode({
  ready: true, browserSurface: 'chat', busy: false, session: liveSession, snapshot: liveSnapshot,
}), 'official_live')
assert.equal(selectLocalAiAnswerRenderMode({
  ready: true, browserSurface: 'chat', busy: false,
  session: { ...liveSession, semanticCacheStatus: 'cached' }, snapshot: liveSnapshot,
}), 'native_cache')
assert.equal(selectLocalAiAnswerRenderMode({
  ready: true, browserSurface: 'official', busy: false, session: liveSession, snapshot: liveSnapshot,
}), 'native')
assert.equal(selectLocalAiAnswerRenderMode({
  ready: true, browserSurface: 'chat', busy: false,
  session: { ...liveSession, contextReady: undefined }, snapshot: liveSnapshot,
}), 'native')
assert.equal(selectLocalAiAnswerRenderMode({
  ready: true, browserSurface: 'chat', busy: false, session: liveSession,
  snapshot: { ...liveSnapshot, streaming: true },
}), 'native')

assert.match(surface, /\{ contentOnly: true \}/)
assert.match(surface, /ResizeObserver/)
assert.match(surface, /setFailedKey\(answerKey\)/)
assert.match(surface, /hideLocalAiWebSessionEmbedded/)
assert.match(surface, /AI_BROWSER_SURFACE_CHANGED_EVENT/)
assert.doesNotMatch(surface, /dismissedKey|REQUEST_RETURN_TO_AI_CHAT_EVENT/)
assert.match(surface, /if \(next === 'chat'\) setFailedKey\(''\)/)
assert.match(backend, /if \(part\.type !== 'image'\) return true/)
assert.match(backend, /part\.url \|\| part\.mediaType \|\| \(label && label !== '图片'\)/)

const contentOnlyBranch = api.slice(
  api.indexOf('export async function presentLocalAiWebSessionEmbedded'),
  api.indexOf('export function announceAiBrowserSurface'),
)
assert.match(contentOnlyBranch, /if \(!options\.contentOnly\)/)
assert.match(contentOnlyBranch, /await waitForOfficialPage\(request\)/)
assert.match(contentOnlyBranch, /contentOnly: options\.contentOnly === true/)
assert.match(api, /officialSurfaceQueue\.then\(work, work\)/)
assert.match(api, /return queueOfficialSurface\(\(\) => invoke<LocalAiWebSessionState>\('hide_local_ai_web_session_embedded'/)
assert.match(fullPage, /\{ contentOnly: false \}/)

assert.match(embedded, /content_only: Option<bool>/)
assert.match(embedded, /if content_only[\s\S]*answer_surface_ready/)
assert.match(embedded, /main, \[role="main"\]/)
assert.match(embedded, /data-elon-official-answer-root/)
assert.match(embedded, /data-elon-official-answer-provider/)
assert.match(embedded, /chatgpt\\\.com/)
assert.match(embedded, /max-width: 48rem/)
assert.match(embedded, /set_content_surface_mode\(&webview, false\)/)
assert.doesNotMatch(surface, /android|pwa/i)

console.log('PASS: Win chat automatically uses the live official answer surface with native cache fallback')

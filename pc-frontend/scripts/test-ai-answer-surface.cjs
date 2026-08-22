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
const controller = read('pc-frontend/src/features/user-browser/useLocalAiWebChatController.ts')
const chatGptAdapter = read('android/app/src/main/assets/chatgpt_web_adapter.js')
const chatGptMessages = read('android/app/src/main/assets/chatgpt_web_adapter_messages.js')
const structuredPolicy = read('pc-frontend/src/features/user-browser/localAiStructuredPartPolicy.ts')
const structuredContent = read('pc-frontend/src/features/ai/AiStructuredContent.tsx')
const messageRow = read('pc-frontend/src/features/ai/AiChatMessageRow.tsx')
const markdownContent = read('pc-frontend/src/features/markdown/MarkdownContent.tsx')
const markdownStyles = read('pc-frontend/src/features/markdown/MarkdownContent.module.css')
const visibility = read('pc-frontend/src/features/ai/aiMessageVisibility.ts')
const api = read('pc-frontend/src/features/user-browser/internalBrowserApi.ts')
const fullPage = read('pc-frontend/src/features/user-browser/AiBrowserExperience.tsx')
const embedded = read('desktop-shell/src-tauri/src/local_ai_browser/embedded_view.rs')

assert.match(page, /<AiOfficialAnswerSurface web=\{web\} \/>/)
assert.match(pageStyles, /\.feed\s*\{[^}]*position:\s*relative;/s)

assert.match(policy, /semanticCacheStatus === 'cached'/)
assert.match(policy, /semanticCacheStatus !== 'live'/)
assert.match(policy, /browserSurface !== 'chat'/)
assert.doesNotMatch(policy, /contextReady !== true/)
assert.match(policy, /snapshot\.streaming/)
assert.match(policy, /responseStreaming/)
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
}), 'official_live')
assert.equal(selectLocalAiAnswerRenderMode({
  ready: true, browserSurface: 'chat', busy: false,
  session: { ...liveSession, lastError: '上一次展示区域的瞬时错误' }, snapshot: liveSnapshot,
}), 'official_live')
assert.equal(selectLocalAiAnswerRenderMode({
  ready: true, browserSurface: 'chat', busy: false, session: liveSession,
  snapshot: { ...liveSnapshot, streaming: true },
}), 'native')
assert.equal(selectLocalAiAnswerRenderMode({
  ready: true, browserSurface: 'chat', busy: false, responseStreaming: true,
  session: liveSession, snapshot: liveSnapshot,
}), 'native')

assert.match(surface, /\{ contentOnly: true \}/)
assert.match(surface, /responseStreaming: Boolean\(web\.streamingMessageId\)/)
assert.match(surface, /ResizeObserver/)
assert.match(surface, /setFailedKey\(presentationKey\)/)
assert.match(surface, /MAX_PRESENT_ATTEMPTS\s*=\s*4/)
assert.doesNotMatch(surface, /web\.controller\.sessionState\?\.updatedAtMs/)
assert.match(surface, /void synchronize\(\)/)
assert.match(surface, /window\.clearTimeout\(retryTimer\)/)
assert.match(surface, /hideLocalAiWebSessionEmbedded/)
assert.match(surface, /AI_BROWSER_SURFACE_CHANGED_EVENT/)
assert.doesNotMatch(surface, /dismissedKey|REQUEST_RETURN_TO_AI_CHAT_EVENT/)
assert.match(surface, /if \(next === 'chat'\) \{[\s\S]*setFailedKey\(''\)/)
assert.match(backend, /\.filter\(shouldRenderNativeStructuredPart\)/)
assert.match(backend, /\[\.\.\.controller\.visibleMessages\]/)
assert.match(controller, /beginPendingLocalAiResponse/)
assert.match(controller, /pendingLocalAiResponseObserved/)
assert.match(chatGptAdapter, /messageAdapter\.lastAssistantPending\(\)/)
assert.match(chatGptMessages, /THINKING_CURSOR_PLACEHOLDERS/)
assert.match(chatGptMessages, /lastAssistantPending/)
assert.match(structuredContent, /part\.type !== 'image'/)
assert.match(structuredContent, /visibleParts\.map/)
assert.match(messageRow, /hasVisibleAiMessageContent\(content\)/)
assert.match(messageRow, /!streaming && hasVisibleContent/)
assert.match(markdownContent, /citationIndex/)
assert.match(markdownContent, /findCitation\(citationIndex, safe\)/)
assert.match(markdownContent, /className=\{styles\.citationLink\}/)
assert.match(markdownStyles, /\.citationLink\s*\{/)

const visibilityFilename = path.join(root, 'pc-frontend/src/features/ai/aiMessageVisibility.ts')
const visibilityOutput = ts.transpileModule(visibility, {
  compilerOptions: { module: ts.ModuleKind.CommonJS, target: ts.ScriptTarget.ES2020 },
  fileName: visibilityFilename,
}).outputText
const compiledVisibility = new Module(visibilityFilename, module)
compiledVisibility.filename = visibilityFilename
compiledVisibility.paths = module.paths
compiledVisibility._compile(visibilityOutput, visibilityFilename)
const { hasVisibleAiMessageContent, shouldKeepAiWebMessage } = compiledVisibility.exports
assert.equal(hasVisibleAiMessageContent(''), false)
assert.equal(hasVisibleAiMessageContent(' \n\t\u200b\u200c\u200d\u2060\ufeff '), false)
assert.equal(hasVisibleAiMessageContent('\u258d\u2026\ue000'), false)
assert.equal(hasVisibleAiMessageContent('正在回答'), true)
assert.equal(hasVisibleAiMessageContent('**正文**'), true)
assert.equal(shouldKeepAiWebMessage({ content: '\u200b', state: 'streaming' }), true)
assert.equal(shouldKeepAiWebMessage({ content: '\u200b', state: 'completed' }), false)
assert.equal(shouldKeepAiWebMessage({ content: '', state: 'completed', sourceCount: 1 }), true)

const structuredPolicyFilename = path.join(root, 'pc-frontend/src/features/user-browser/localAiStructuredPartPolicy.ts')
const structuredPolicyOutput = ts.transpileModule(structuredPolicy, {
  compilerOptions: { module: ts.ModuleKind.CommonJS, target: ts.ScriptTarget.ES2020 },
  fileName: structuredPolicyFilename,
}).outputText
const compiledStructuredPolicy = new Module(structuredPolicyFilename, module)
compiledStructuredPolicy.filename = structuredPolicyFilename
compiledStructuredPolicy.paths = module.paths
compiledStructuredPolicy._compile(structuredPolicyOutput, structuredPolicyFilename)
const { shouldRenderNativeStructuredPart } = compiledStructuredPolicy.exports
assert.equal(shouldRenderNativeStructuredPart({ type: 'image', text: '图片', mediaType: 'image/webp' }), false)
assert.equal(shouldRenderNativeStructuredPart({ type: 'image', text: ' 图片\u200b图片 ', url: 'https://example.com/a.png' }), false)
assert.equal(shouldRenderNativeStructuredPart({ type: 'image', text: 'NVIDIA 盘前走势图' }), false)
assert.equal(shouldRenderNativeStructuredPart({ type: 'table', text: '行情表格' }), true)

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
const answerSurfaceGate = embedded.slice(
  embedded.indexOf('fn answer_surface_ready'),
  embedded.indexOf('fn semantic_event_has_completed_assistant'),
)
assert.doesNotMatch(answerSurfaceGate, /context_ready/)
assert.match(embedded, /main, \[role="main"\]/)
assert.match(embedded, /data-elon-official-answer-root/)
assert.match(embedded, /data-elon-official-answer-provider/)
assert.match(embedded, /chatgpt\\\.com/)
assert.match(embedded, /max-width: 48rem/)
assert.match(embedded, /set_content_surface_mode\(&webview, false\)/)
assert.doesNotMatch(surface, /android|pwa/i)

console.log('PASS: Win chat automatically uses the live official answer surface with native cache fallback')

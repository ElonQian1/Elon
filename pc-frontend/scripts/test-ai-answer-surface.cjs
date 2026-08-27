const assert = require('node:assert/strict')
const fs = require('node:fs')
const Module = require('node:module')
const path = require('node:path')
const ts = require('typescript')

const root = path.resolve(__dirname, '..', '..')
const read = (relativePath) => fs.readFileSync(path.join(root, relativePath), 'utf8')

const page = read('pc-frontend/src/features/ai/AiChatPage.tsx')
const pageStyles = read('pc-frontend/src/features/ai/AiChatPage.module.css')
const backend = read('pc-frontend/src/features/user-browser/useAiWebChatBackend.ts')
const controller = read('pc-frontend/src/features/user-browser/useLocalAiWebChatController.ts')
const responseRefresh = read('pc-frontend/src/features/user-browser/useLocalAiResponseRefresh.ts')
const responseRefreshFlight = read('pc-frontend/src/features/user-browser/localAiResponseRefreshFlight.ts')
const chatGptAdapter = read('android/app/src/main/assets/chatgpt_web_adapter.js')
const chatGptMessages = read('android/app/src/main/assets/chatgpt_web_adapter_messages.js')
const chatGptStreamingPolicy = read('android/app/src/main/assets/chatgpt_web_adapter_streaming_policy.js')
const structuredPolicy = read('pc-frontend/src/features/user-browser/localAiStructuredPartPolicy.ts')
const structuredContent = read('pc-frontend/src/features/ai/AiStructuredContent.tsx')
const messageRow = read('pc-frontend/src/features/ai/AiChatMessageRow.tsx')
const markdownContent = read('pc-frontend/src/features/markdown/MarkdownContent.tsx')
const markdownStyles = read('pc-frontend/src/features/markdown/MarkdownContent.module.css')
const visibility = read('pc-frontend/src/features/ai/aiMessageVisibility.ts')
const api = read('pc-frontend/src/features/user-browser/internalBrowserApi.ts')
const fullPage = read('pc-frontend/src/features/user-browser/AiBrowserExperience.tsx')
const embedded = read('desktop-shell/src-tauri/src/local_ai_browser/embedded_view.rs')
const streamingPresentation = read('pc-frontend/src/features/user-browser/localAiStreamingPresentation.ts')
assert.doesNotMatch(page, /AiOfficialAnswerSurface/)
assert.match(pageStyles, /\.feed\s*\{[^}]*position:\s*relative;/s)
assert.match(page, /visibleMessages\.filter\(\(m\) => m\.role !== 'system'\)\.map/)
assert.match(page, /<AiChatMessageRow/)
assert.match(backend, /\.filter\(shouldRenderNativeStructuredPart\)/)
assert.match(backend, /\[\.\.\.controller\.visibleMessages\]/)
assert.match(backend, /const effectiveState = item\.id === streamingTarget\?\.messageId/)
assert.match(backend, /streamingTarget\?\.synthetic/)
assert.match(backend, /localAiStreamingStatus\(\{/)
assert.match(controller, /beginPendingLocalAiResponse/)
assert.match(controller, /pendingLocalAiResponseObserved/)
assert.match(responseRefresh, /requestReturnToAiChat/)
assert.match(responseRefresh, /controlLocalAiWebSession\(activeProvider\.id, ownerKey, 'background'\)/)
assert.match(responseRefresh, /RESPONSE_COMPLETION_SETTLE_MS/)
assert.match(responseRefresh, /completionObservedAt/)
assert.match(responseRefresh, /requestRef\.current/)
assert.match(responseRefresh, /latestLocalAiAssistantForUserTurn/)
assert.match(responseRefresh, /!localAiAssistantExtractionIncomplete\(assistant\)/)
assert.match(responseRefresh, /requestLocalAiCurrentConversationRefresh/)
assert.match(responseRefresh, /shouldRequestLocalAiPrivateConversationRefresh/)
assert.match(responseRefresh, /LocalAiResponseRefreshFlight/)
assert.match(responseRefresh, /Promise\.allSettled\(requests\)/)
assert.match(responseRefresh, /settlement === 'rerun'/)
assert.match(responseRefreshFlight, /many watchdog ticks|Repeated watchdog ticks/)
assert.match(chatGptAdapter, /streamingPolicyModule\.readState/)
assert.match(chatGptStreamingPolicy, /messageAdapter\.lastAssistantPending\(\)/)
assert.match(chatGptStreamingPolicy, /completionQuietMs/)
assert.match(chatGptMessages, /THINKING_CURSOR_PLACEHOLDERS/)
assert.match(chatGptMessages, /lastAssistantPending/)
assert.match(structuredContent, /part\.type !== 'image'/)
assert.match(structuredContent, /visibleParts\.map/)
assert.match(messageRow, /hasVisibleAiMessageContent\(content\)/)
assert.match(messageRow, /Boolean\(message\.structured_parts\?\.length\)/)
assert.match(messageRow, /!streaming && hasVisibleText/)
assert.match(messageRow, /isLocalAiSearchProgress\(streamingStatus\)/)
assert.match(messageRow, /<Globe2/)
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
assert.equal(hasVisibleAiMessageContent('###### ChatGPT 说：\ue000'), false)
assert.equal(hasVisibleAiMessageContent('ChatGPT said:'), false)
assert.equal(hasVisibleAiMessageContent('正在回答'), true)
assert.equal(hasVisibleAiMessageContent('**正文**'), true)
assert.equal(shouldKeepAiWebMessage({ content: '\u200b', state: 'streaming' }), true)
assert.equal(shouldKeepAiWebMessage({ content: '\u200b', state: 'completed' }), false)
assert.equal(shouldKeepAiWebMessage({ content: '', state: 'completed', sourceCount: 1 }), true)

const streamingPresentationFilename = path.join(root, 'pc-frontend/src/features/user-browser/localAiStreamingPresentation.ts')
const streamingPresentationOutput = ts.transpileModule(streamingPresentation, {
  compilerOptions: { module: ts.ModuleKind.CommonJS, target: ts.ScriptTarget.ES2023 },
  fileName: streamingPresentationFilename,
}).outputText
const compiledStreamingPresentation = new Module(streamingPresentationFilename, module)
compiledStreamingPresentation.filename = streamingPresentationFilename
compiledStreamingPresentation.paths = module.paths
compiledStreamingPresentation._compile(streamingPresentationOutput, streamingPresentationFilename)
const {
  isLocalAiSearchProgress,
  localAiStreamingStatus,
  localAiStreamingTarget,
} = compiledStreamingPresentation.exports
const user = { id: 'user-1', role: 'user', state: 'completed', content: [{ type: 'text', text: 'KOSPI' }] }
const completedShell = { id: 'assistant-shell', role: 'assistant', state: 'completed', content: [] }
assert.deepEqual(localAiStreamingTarget([user, completedShell], true), {
  messageId: 'assistant-shell', synthetic: false,
})
assert.deepEqual(localAiStreamingTarget([user], true), {
  messageId: 'snapshot-progress', synthetic: true,
})
assert.equal(localAiStreamingTarget([user, completedShell], false), null)
assert.equal(localAiStreamingTarget([
  user,
  { ...completedShell, id: 'stale-private-shell', state: 'streaming' },
], true, 'completed'), null, 'private completion must close a stale DOM streaming shell')
assert.deepEqual(localAiStreamingTarget([user], false, 'streaming'), {
  messageId: 'snapshot-progress', synthetic: true,
}, 'private stream activity must render progress before the DOM assistant appears')
assert.deepEqual(localAiStreamingTarget([
  { ...completedShell, id: 'old-stream', state: 'streaming' },
  user,
], true), { messageId: 'snapshot-progress', synthetic: true }, 'an old turn must not own the new stream')
assert.equal(localAiStreamingTarget([
  user,
  { ...completedShell, id: 'assistant-live', state: 'streaming' },
], false).messageId, 'assistant-live')
assert.equal(localAiStreamingStatus({
  officialStatus: '正在搜索 South Korea stock market', pendingSlow: true, providerName: 'ChatGPT',
}), '正在搜索 South Korea stock market', 'official progress must beat the generic slow warning')
assert.match(localAiStreamingStatus({ pendingSlow: true, providerName: 'ChatGPT' }), /回答同步较慢/)
assert.equal(isLocalAiSearchProgress('正在搜索 11 个网站'), true)
assert.equal(isLocalAiSearchProgress('ChatGPT 正在回答…'), false)

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

const officialPageBranch = api.slice(
  api.indexOf('export async function presentLocalAiWebSessionEmbedded'),
  api.indexOf('export function announceAiBrowserSurface'),
)
assert.match(officialPageBranch, /await waitForOfficialPage\(request\)/)
assert.doesNotMatch(officialPageBranch, /contentOnly/)
assert.match(api, /officialSurfaceQueue\.then\(work, work\)/)
assert.match(api, /officialSurfaceIntentVersion/)
assert.match(api, /officialSurfaceRequestedVisible = false/)
assert.match(api, /officialSurfaceIntentIsCurrent\(intentVersion, true\)/)
assert.match(api, /return queueOfficialSurface\(\(\) => hideOfficialSessionNow\(request\)\)/)
assert.match(fullPage, /presentLocalAiWebSessionEmbedded\(official, bounds\)/)
assert.match(fullPage, /foreground ownership command/)
assert.match(fullPage, /hideOfficialSurface\(activeOfficial\)/)
assert.doesNotMatch(fullPage, /contentOnly/)

assert.doesNotMatch(embedded, /content_only|answer_surface|set_content_surface_mode/)
assert.match(embedded, /present\(&app, &label, bounds\)/)

const primaryCardIndex = messageRow.indexOf('placement="primary"')
const markdownIndex = messageRow.indexOf('<MarkdownContent')
const sourceIndex = messageRow.indexOf('<AiSourceLinks')
const supplementaryCardIndex = messageRow.indexOf('placement="supplementary"')
assert.ok(primaryCardIndex >= 0 && primaryCardIndex < markdownIndex)
assert.ok(markdownIndex < sourceIndex && sourceIndex < supplementaryCardIndex)
assert.match(structuredContent, /content\.kind === 'finance' \|\| content\.kind === 'chart'/)
assert.match(structuredContent, /content\.kind === 'weather' \|\| content\.kind === 'map'/)

console.log('PASS: Win chat keeps native conversation turns primary and opens the full official page explicitly')

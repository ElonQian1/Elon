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
assert.doesNotMatch(page, /AiOfficialAnswerSurface/)
assert.match(pageStyles, /\.feed\s*\{[^}]*position:\s*relative;/s)
assert.match(page, /visibleMessages\.filter\(\(m\) => m\.role !== 'system'\)\.map/)
assert.match(page, /<AiChatMessageRow/)
assert.match(backend, /\.filter\(shouldRenderNativeStructuredPart\)/)
assert.match(backend, /\[\.\.\.controller\.visibleMessages\]/)
assert.match(controller, /beginPendingLocalAiResponse/)
assert.match(controller, /pendingLocalAiResponseObserved/)
assert.match(chatGptAdapter, /streamingPolicyModule\.readState/)
assert.match(chatGptStreamingPolicy, /messageAdapter\.lastAssistantPending\(\)/)
assert.match(chatGptStreamingPolicy, /completionQuietMs/)
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

const officialPageBranch = api.slice(
  api.indexOf('export async function presentLocalAiWebSessionEmbedded'),
  api.indexOf('export function announceAiBrowserSurface'),
)
assert.match(officialPageBranch, /await waitForOfficialPage\(request\)/)
assert.doesNotMatch(officialPageBranch, /contentOnly/)
assert.match(api, /officialSurfaceQueue\.then\(work, work\)/)
assert.match(api, /return queueOfficialSurface\(\(\) => invoke<LocalAiWebSessionState>\('hide_local_ai_web_session_embedded'/)
assert.match(fullPage, /presentLocalAiWebSessionEmbedded\(official, bounds\)/)
assert.doesNotMatch(fullPage, /contentOnly/)

assert.doesNotMatch(embedded, /content_only|answer_surface|set_content_surface_mode/)
assert.match(embedded, /present\(&app, &label, bounds\)/)

console.log('PASS: Win chat keeps native conversation turns primary and opens the full official page explicitly')

const assert = require('node:assert/strict')
const fs = require('node:fs')
const Module = require('node:module')
const path = require('node:path')
const ts = require('typescript')

const filename = path.resolve(__dirname, '../src/features/user-browser/localAiComposerAvailability.ts')
const source = fs.readFileSync(filename, 'utf8')
const output = ts.transpileModule(source, {
  compilerOptions: { module: ts.ModuleKind.CommonJS, target: ts.ScriptTarget.ES2020 },
  fileName: filename,
}).outputText
const compiled = new Module(filename, module)
compiled.filename = filename
compiled.paths = module.paths
compiled._compile(output, filename)
const { localAiComposerAvailability } = compiled.exports

function loadTypeScriptModule(relativePath) {
  const target = path.resolve(__dirname, relativePath)
  const result = ts.transpileModule(fs.readFileSync(target, 'utf8'), {
    compilerOptions: { module: ts.ModuleKind.CommonJS, target: ts.ScriptTarget.ES2020 },
    fileName: target,
  }).outputText
  const loaded = new Module(target, module)
  loaded.filename = target
  loaded.paths = module.paths
  loaded._compile(result, target)
  return loaded.exports
}

const {
  LocalAiProviderDraftCache,
  LOCAL_AI_PROVIDER_DRAFT_MAX_LENGTH,
  localAiProviderDraftIdentity,
} = loadTypeScriptModule('../src/features/user-browser/localAiProviderDraftCache.ts')
const { localAiWarmSessionReusable } = loadTypeScriptModule(
  '../src/features/user-browser/localAiWarmSessionPolicy.ts',
)

const base = {
  clientReady: true,
  providerAvailable: true,
  sendSupported: true,
  directSendReady: false,
  newConversationRecoveryActive: false,
  queuedSendActive: false,
  sendFlightActive: false,
  busyAction: '',
}

assert.deepEqual(localAiComposerAvailability(base), {
  canEdit: true,
  canSubmit: false,
  shouldQueue: false,
})
assert.deepEqual(localAiComposerAvailability({
  ...base,
  newConversationRecoveryActive: true,
  busyAction: 'new_conversation',
}), {
  canEdit: true,
  canSubmit: true,
  shouldQueue: true,
})
assert.equal(localAiComposerAvailability({
  ...base,
  newConversationRecoveryActive: true,
  queuedSendActive: true,
}).canSubmit, false)
assert.deepEqual(localAiComposerAvailability({ ...base, directSendReady: true }), {
  canEdit: true,
  canSubmit: true,
  shouldQueue: false,
})
assert.deepEqual(localAiComposerAvailability({
  ...base,
  directSendReady: true,
  sendFlightActive: true,
}), {
  canEdit: true,
  canSubmit: false,
  shouldQueue: false,
})
assert.deepEqual(localAiComposerAvailability({
  ...base,
  directSendReady: true,
  newConversationRecoveryActive: true,
}), {
  canEdit: true,
  canSubmit: true,
  shouldQueue: true,
})
assert.deepEqual(localAiComposerAvailability({
  ...base,
  directSendReady: true,
  busyAction: 'send_prompt',
}), {
  canEdit: true,
  canSubmit: false,
  shouldQueue: false,
})
assert.equal(localAiComposerAvailability({ ...base, clientReady: false }).canEdit, false)

const drafts = new LocalAiProviderDraftCache(2)
const pendingChatGpt = localAiProviderDraftIdentity('chatgpt-web', '')
const ownedChatGpt = localAiProviderDraftIdentity('chatgpt-web', 'owner-a')
const ownedGoogle = localAiProviderDraftIdentity('google-ai-mode', 'owner-a')
drafts.remember(pendingChatGpt, 'typed before identity recovery')
assert.equal(drafts.claimPending('chatgpt-web', 'owner-a'), 'typed before identity recovery')
assert.equal(drafts.read(pendingChatGpt), '')
drafts.remember(ownedGoogle, 'Google draft')
assert.equal(drafts.read(ownedChatGpt), 'typed before identity recovery')
assert.equal(drafts.read(ownedGoogle), 'Google draft')
drafts.remember(ownedGoogle, 'x'.repeat(LOCAL_AI_PROVIDER_DRAFT_MAX_LENGTH + 20))
assert.equal(drafts.read(ownedGoogle).length, LOCAL_AI_PROVIDER_DRAFT_MAX_LENGTH)

const warmState = {
  providerId: 'chatgpt',
  windowStatus: 'minimized',
  lastError: null,
}
assert.equal(localAiWarmSessionReusable(warmState, 'chatgpt'), true)
assert.equal(localAiWarmSessionReusable({ ...warmState, providerId: 'google-ai-mode' }, 'chatgpt'), false)
assert.equal(localAiWarmSessionReusable({ ...warmState, windowStatus: 'closed' }, 'chatgpt'), false)
assert.equal(localAiWarmSessionReusable({ ...warmState, windowStatus: 'blocked' }, 'chatgpt'), false)
assert.equal(localAiWarmSessionReusable({ ...warmState, lastError: 'page failed' }, 'chatgpt'), false)

const composerDraft = fs.readFileSync(
  path.resolve(__dirname, '../src/features/user-browser/useLocalAiComposerDraft.ts'),
  'utf8',
)
assert.match(composerDraft, /claimPending\(providerId, ownerKey\)/)
assert.match(composerDraft, /localAiProviderDraftCache\.remember\(activeIdentity\.current, next\)/)

const page = fs.readFileSync(path.resolve(__dirname, '../src/features/ai/AiChatPage.tsx'), 'utf8')
const backend = fs.readFileSync(path.resolve(__dirname, '../src/features/user-browser/useAiWebChatBackend.ts'), 'utf8')
const controller = fs.readFileSync(
  path.resolve(__dirname, '../src/features/user-browser/useLocalAiWebChatController.ts'),
  'utf8',
)
assert.match(page, /disabled=\{chatMode \? !web\.canEdit : visibleSending\}/)
assert.match(page, /busyAction !== 'new_conversation'/)
assert.match(page, /requestAnimationFrame\(\(\) => textareaRef\.current\?\.focus\(\)\)/)
assert.match(page, /className=\{styles\.newBtn\} onClick=\{newConversation\}/)
assert.match(backend, /const canEdit = capability\.state === 'ready' && Boolean\(provider\) && controller\.canEditDraft/)
assert.match(backend, /const canCompose = ready && controller\.canSubmitDraft/)
assert.match(backend, /官网页面正在后台异步同步/)
assert.match(controller, /resumeLocalAiWebSession\(providerId, ownerKey, cachedState\)/)
assert.match(controller, /if \(!warmSession\) setMessage/)
assert.match(controller, /sendFlightActive: pendingSends\.length > 0 \|\| pendingResponses\.length > 0/)
assert.match(controller, /action === 'send_prompt' && !composerAvailability\.canSubmit/)

process.stdout.write('PASS local AI non-blocking composer availability\n')

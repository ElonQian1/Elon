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

const base = {
  clientReady: true,
  providerAvailable: true,
  sendSupported: true,
  directSendReady: false,
  newConversationRecoveryActive: false,
  queuedSendActive: false,
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
  busyAction: 'send_prompt',
}), {
  canEdit: true,
  canSubmit: false,
  shouldQueue: false,
})
assert.equal(localAiComposerAvailability({ ...base, clientReady: false }).canEdit, false)

const page = fs.readFileSync(path.resolve(__dirname, '../src/features/ai/AiChatPage.tsx'), 'utf8')
const backend = fs.readFileSync(path.resolve(__dirname, '../src/features/user-browser/useAiWebChatBackend.ts'), 'utf8')
assert.match(page, /disabled=\{chatMode \? !web\.canEdit : visibleSending\}/)
assert.match(page, /busyAction !== 'new_conversation'/)
assert.match(backend, /const canEdit = ready && controller\.canEditDraft/)
assert.match(backend, /const canCompose = ready && controller\.canSubmitDraft/)
assert.match(backend, /官网页面正在后台异步同步/)

process.stdout.write('PASS local AI non-blocking composer availability\n')

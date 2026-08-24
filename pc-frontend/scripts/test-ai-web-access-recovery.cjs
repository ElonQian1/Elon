const assert = require('node:assert/strict')
const fs = require('node:fs')
const path = require('node:path')

const component = fs.readFileSync(path.resolve(
  __dirname,
  '../src/features/user-browser/AiWebAccessRecoveryCard.tsx',
), 'utf8')
const controls = fs.readFileSync(path.resolve(
  __dirname,
  '../src/features/user-browser/AiWebComposerControls.tsx',
), 'utf8')
const controller = fs.readFileSync(path.resolve(
  __dirname,
  '../src/features/user-browser/useLocalAiWebChatController.ts',
), 'utf8')
const recovery = fs.readFileSync(path.resolve(
  __dirname,
  '../src/features/user-browser/useLocalAiAccessRecovery.ts',
), 'utf8')

assert.match(component, /data-testid="ai-web-access-recovery"/)
assert.match(component, /新对话重试/)
assert.match(component, /显示官方页/)
assert.match(component, /暂不重试/)
assert.match(component, /只有点击重试才会再次提交/)
assert.match(controls, /<AiWebAccessRecoveryCard web=\{web\} \/>/)
assert.match(recovery, /cancelResponseRefresh\(\)/)
assert.match(recovery, /clearPendingResponses\(\[\]\)/)
assert.match(recovery, /accessReason === 'login_required'/)
assert.match(controller, /retryLoginBlockedPrompt/)

process.stdout.write('PASS AI web access recovery card and explicit retry contract\n')

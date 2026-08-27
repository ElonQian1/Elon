const assert = require('node:assert/strict')
const fs = require('node:fs')
const Module = require('node:module')
const path = require('node:path')
const ts = require('typescript')

const filename = path.resolve(
  __dirname,
  '../src/features/user-browser/useLocalAiBackgroundNavigationRecovery.ts',
)
const source = fs.readFileSync(filename, 'utf8')
const output = ts.transpileModule(source, {
  compilerOptions: { module: ts.ModuleKind.CommonJS, target: ts.ScriptTarget.ES2020 },
  fileName: filename,
}).outputText
const compiled = new Module(filename, module)
compiled.filename = filename
compiled.paths = module.paths
compiled.require = (id) => id === 'react'
  ? { useEffect: () => {}, useRef: (current) => ({ current }) }
  : id === './localAiBrowserApi'
    ? {}
    : require(id)
compiled._compile(output, filename)

const {
  LOCAL_AI_BACKGROUND_STALL_TIMEOUT_MS,
  localAiBackgroundNavigationStalled,
} = compiled.exports
const loading = {
  loading: true,
  rendererStatus: 'connecting',
  windowStatus: 'loading',
  windowVisible: false,
}

assert.equal(LOCAL_AI_BACKGROUND_STALL_TIMEOUT_MS, 12_000)
assert.equal(localAiBackgroundNavigationStalled(loading, false), true)
assert.equal(localAiBackgroundNavigationStalled({ ...loading, windowVisible: true }, false), false)
assert.equal(localAiBackgroundNavigationStalled({ ...loading, loading: false }, false), false)
assert.equal(localAiBackgroundNavigationStalled({ ...loading, rendererStatus: 'active' }, false), false)
assert.equal(localAiBackgroundNavigationStalled(loading, true), false)
assert.match(source, /attemptedGeneration\.current === generationKey/)
assert.match(source, /getLocalAiWebSessionState\(providerId, ownerKey\)/)
assert.match(source, /attemptedGeneration\.current = generationKey[\s\S]*?controlLocalAiWebSession\(providerId, ownerKey, 'reload'\)/)
assert.match(source, /window\.setTimeout\([\s\S]*?LOCAL_AI_BACKGROUND_STALL_TIMEOUT_MS/)
assert.doesNotMatch(source, /setInterval/)

const controller = fs.readFileSync(path.resolve(
  __dirname,
  '../src/features/user-browser/useLocalAiWebChatController.ts',
), 'utf8')
assert.match(controller, /useLocalAiBackgroundNavigationRecovery\(/)
assert.match(controller, /pendingSends\.length \|\| pendingResponses\.length/)
assert.match(controller, /newConversationRecoveryStartedAtMs \|\| userState\.phase === 'streaming'/)

process.stdout.write('PASS local AI bounded background navigation recovery\n')

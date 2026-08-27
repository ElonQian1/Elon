const assert = require('node:assert/strict')
const fs = require('node:fs')
const Module = require('node:module')
const path = require('node:path')
const ts = require('typescript')

const filename = path.resolve(
  __dirname,
  '../src/features/user-browser/resumeLocalAiWebSession.ts',
)
const source = fs.readFileSync(filename, 'utf8')

function loadResume(api) {
  const output = ts.transpileModule(source, {
    compilerOptions: { module: ts.ModuleKind.CommonJS, target: ts.ScriptTarget.ES2020 },
    fileName: filename,
  }).outputText
  const compiled = new Module(filename, module)
  compiled.filename = filename
  compiled.paths = module.paths
  compiled.require = (request) => {
    if (request === './localAiBrowserApi') return api
    if (request === './localAiWarmSessionPolicy') {
      return {
        localAiWarmSessionReusable: (state, providerId) => Boolean(
          state
            && state.providerId === providerId
            && !['closed', 'blocked', 'error'].includes(state.windowStatus)
            && !state.lastError,
        ),
      }
    }
    return Module.prototype.require.call(compiled, request)
  }
  compiled._compile(output, filename)
  return compiled.exports.resumeLocalAiWebSession
}

async function main() {
  const durable = { providerId: 'chatgpt', windowStatus: 'closed', lastError: null, title: 'cached' }
  const live = { ...durable, windowStatus: 'minimized', title: 'live' }
  const events = []
  let releaseOpen
  const openGate = new Promise((resolve) => { releaseOpen = resolve })
  let reads = 0
  const resume = loadResume({
    getLocalAiWebSessionState: async () => {
      events.push(`get:${reads}`)
      reads += 1
      return reads === 1 ? durable : live
    },
    openLocalAiWebSession: async () => {
      events.push('open')
      await openGate
    },
  })
  let published = null
  const pending = resume('chatgpt', 'owner', null, (state) => {
    events.push('publish-cache')
    published = state
  })
  await new Promise((resolve) => setImmediate(resolve))
  assert.equal(published, durable, 'native durable snapshot must publish before WebView open finishes')
  assert.deepEqual(events, ['get:0', 'publish-cache', 'open'])
  releaseOpen()
  assert.deepEqual(await pending, { state: live, reused: false })

  const warm = { ...live, title: 'already-open' }
  let opened = false
  const reuse = loadResume({
    getLocalAiWebSessionState: async () => warm,
    openLocalAiWebSession: async () => { opened = true },
  })
  assert.deepEqual(await reuse('chatgpt', 'owner', warm), { state: warm, reused: true })
  assert.equal(opened, false)

  process.stdout.write('PASS local AI native cache-first session resume\n')
}

main().catch((error) => {
  console.error(error)
  process.exitCode = 1
})

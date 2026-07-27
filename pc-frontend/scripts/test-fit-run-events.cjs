const assert = require('node:assert/strict')
const fs = require('node:fs')
const os = require('node:os')
const path = require('node:path')
const ts = require('typescript')

const projectRoot = path.resolve(__dirname, '..')
const temporaryDirectory = fs.mkdtempSync(path.join(os.tmpdir(), 'elon-fit-run-events-'))
const sourceFile = path.join(projectRoot, 'src/features/ui-tuner/fit-run/fitRunEvents.ts')
const outputFile = path.join(temporaryDirectory, 'fitRunEvents.js')
const compiled = ts.transpileModule(fs.readFileSync(sourceFile, 'utf8'), {
  compilerOptions: { module: ts.ModuleKind.CommonJS, target: ts.ScriptTarget.ES2022 },
  fileName: sourceFile,
})
fs.writeFileSync(outputFile, compiled.outputText)

class TestCustomEvent extends Event {
  constructor(type, init) {
    super(type)
    this.detail = init?.detail
  }
}

const storage = new Map()
const windowTarget = new EventTarget()
windowTarget.setTimeout = setTimeout
windowTarget.clearTimeout = clearTimeout
windowTarget.localStorage = {
  getItem: (key) => storage.get(key) ?? null,
  setItem: (key, value) => storage.set(key, value),
  removeItem: (key) => storage.delete(key),
}
global.window = windowTarget
global.CustomEvent = TestCustomEvent

const {
  listenForFitRunCodexRequests,
  persistFitRunCodexLaunch,
  readFitRunCodexLaunchByRun,
  requestCodexForFitRun,
  resolveFitRunWorkspace,
} = require(outputFile)
const request = { runId: 'run-1', handoffId: 'handoff-1', reason: '测试 AI 接力' }

;(async () => {
  assert.deepEqual(
    resolveFitRunWorkspace({}, 'D:\\project'),
    { workspacePath: 'D:\\project', isOverride: false },
  )
  assert.deepEqual(
    resolveFitRunWorkspace({
      workspacePath: 'D:/project-worktree',
      contextPack: { screen: { sourceRoot: 'd:\\project-worktree\\' } },
    }, 'D:\\project'),
    { workspacePath: 'D:/project-worktree', isOverride: true },
  )
  assert.throws(
    () => resolveFitRunWorkspace({
      workspacePath: 'D:\\wrong-project',
      contextPack: { screen: { sourceRoot: 'D:\\expected-project' } },
    }, 'D:\\project'),
    /目录与 AI Context Artifact 不一致/,
  )

  await assert.rejects(
    requestCodexForFitRun(request, 100),
    /AI 项目会话入口未就绪/,
    '没有常驻桥接器时必须立即失败，不能无限等待',
  )

  const disposeSuccess = listenForFitRunCodexRequests((detail) => {
    detail.resolve({ taskId: 'task-1' })
  })
  assert.deepEqual(await requestCodexForFitRun(request, 100), { taskId: 'task-1' })
  disposeSuccess()

  persistFitRunCodexLaunch({
    runId: 'pwa:project-1:7',
    handoffId: 'pwa_1',
    taskId: 'task-pwa-1',
    handoffKind: 'PWA_DRAFT',
    createdAt: '2026-07-28T00:00:00.000Z',
  })
  assert.equal(
    readFitRunCodexLaunchByRun('pwa:project-1:7', 'PWA_DRAFT').taskId,
    'task-pwa-1',
    '刷新后 PWA 草稿必须能按 runId 恢复上次 Codex 写回任务',
  )

  const disposeTimeout = listenForFitRunCodexRequests(() => undefined)
  await assert.rejects(
    requestCodexForFitRun(request, 10),
    /AI 项目会话启动超时/,
    '桥接器已接管但没有返回 taskId 时必须超时恢复',
  )
  disposeTimeout()

  console.log('fit-run Codex event bridge tests passed')
})().finally(() => {
  fs.rmSync(temporaryDirectory, { recursive: true, force: true })
}).catch((error) => {
  console.error(error)
  process.exitCode = 1
})

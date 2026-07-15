const assert = require('node:assert/strict')
const fs = require('node:fs')
const path = require('node:path')
const Module = require('node:module')

const root = path.resolve(__dirname, '..')
const pcRoot = path.join(root, 'pc-frontend')
const typescriptPath = path.join(pcRoot, 'node_modules', 'typescript')
const ts = fs.existsSync(typescriptPath) ? require(typescriptPath) : require('typescript')

function loadTsModule(relativePath) {
  const filename = path.join(pcRoot, 'src', relativePath)
  const source = fs.readFileSync(filename, 'utf8')
  const output = ts.transpileModule(source, {
    compilerOptions: {
      target: ts.ScriptTarget.ES2020,
      module: ts.ModuleKind.CommonJS,
      esModuleInterop: true,
    },
    fileName: filename,
  }).outputText
  const loaded = new Module(filename, module)
  loaded.filename = filename
  loaded.paths = Module._nodeModulePaths(path.dirname(filename))
  loaded._compile(output, filename)
  return loaded.exports
}

const model = loadTsModule('task-progress-replay/model.ts')
const captures = loadTsModule('task-progress-replay/captures.ts')

const failure = captures.dataRootFailureCapture
const failureFrames = model.buildReplayFrames(failure)
assert.equal(failureFrames.length, 17, '8 messages + 8 raw events + start frame must be preserved')
assert.equal(failureFrames[0].source, 'start')
assert.equal(failureFrames[1].source, 'message')
assert.equal(failureFrames[1].title, '用户发起任务')
assert.equal(failureFrames[failureFrames.length - 1].atMs, 2783)
assert.equal(failureFrames[failureFrames.length - 1].source, 'event')

const beforeResult = failureFrames.findLast
  ? failureFrames.findLast((frame) => frame.messageCount === 7)
  : [...failureFrames].reverse().find((frame) => frame.messageCount === 7)
assert.ok(beforeResult)
const runningMessages = model.replayMessagesAtFrame(failure, beforeResult)
assert.ok(runningMessages.length > 0)
assert.ok(runningMessages.every((message) => message.task_status === 'running'))
assert.ok(runningMessages.every((message) => message.task_error == null))

const resultFrame = failureFrames.find((frame) => frame.title === '最终回复到达')
assert.ok(resultFrame)
const terminalMessages = model.replayMessagesAtFrame(failure, resultFrame)
assert.equal(terminalMessages[terminalMessages.length - 1].task_status, 'failed')
assert.match(terminalMessages[terminalMessages.length - 1].task_error, /统一数据根/)

const keyFrames = model.selectReplayKeyFrames(failureFrames)
assert.ok(keyFrames.length <= 8)
assert.equal(keyFrames[0].index, 0)
assert.equal(keyFrames[keyFrames.length - 1].index, failureFrames.length - 1)
assert.ok(keyFrames.every((frame, index) => index === 0 || frame.index > keyFrames[index - 1].index))

const issues = model.captureReplayIssues(failure, failureFrames)
assert.ok(issues.some((issue) => issue.id === 'rapid-startup-events'))
assert.ok(!issues.some((issue) => issue.severity === 'error'))
assert.equal(model.replayFrameDelay(failureFrames[1], failureFrames[2], 1), 500)
assert.equal(model.replayFrameDelay(failureFrames[1], failureFrames[2], 0.25), 2000)

const successFrames = model.buildReplayFrames(captures.completeSuccessCapture)
assert.equal(successFrames.length, 20)
assert.equal(successFrames[successFrames.length - 1].atMs, 13300)
assert.ok(successFrames.some((frame) => frame.title === 'file_change返回结果'))
assert.ok(successFrames.some((frame) => frame.title === '运行摘要'))
assert.equal(model.captureReplayIssues(captures.completeSuccessCapture, successFrames).filter((issue) => issue.severity === 'error').length, 0)

const config = model.parseReplayPreviewConfig('?source=replay&capture=complete-success&frame=7&speed=4&tools=1&filmstrip=0')
assert.equal(config.enabled, true)
assert.equal(config.captureId, 'complete-success')
assert.equal(config.frame, 7)
assert.equal(config.speed, 4)
assert.equal(config.expandTools, true)
assert.equal(config.filmstrip, false)
assert.equal(model.parseReplayPreviewConfig('?source=live').enabled, false)

const previewSource = fs.readFileSync(path.join(pcRoot, 'src', 'task-progress-preview.tsx'), 'utf8')
const replaySource = fs.readFileSync(path.join(pcRoot, 'src', 'task-progress-replay', 'ReplayPreview.tsx'), 'utf8')
const conversationSource = fs.readFileSync(path.join(pcRoot, 'src', 'task-progress-replay', 'ReplayConversation.tsx'), 'utf8')
const captureHookSource = fs.readFileSync(path.join(pcRoot, 'src', 'task-progress-replay', 'useTaskReplayCapture.ts'), 'utf8')
const progressSurfaceSource = fs.readFileSync(path.join(pcRoot, 'src', 'features', 'dev', 'TaskProgressSurface.tsx'), 'utf8')
const timelineSource = fs.readFileSync(path.join(pcRoot, 'src', 'features', 'dev', 'TaskTimeline.tsx'), 'utf8')
assert.match(previewSource, /parseReplayPreviewConfig/)
assert.match(previewSource, /<ReplayPreview config=\{replayConfig\}/)
assert.match(conversationSource, /<ConversationFeed/)
assert.match(conversationSource, /buildDisplayMessages/)
assert.match(conversationSource, /buildMessageGroups/)
assert.match(replaySource, /selectReplayKeyFrames/)
assert.match(replaySource, /auditReplayDom/)
assert.match(captureHookSource, /snapshot\?since=\$\{since\}&limit=200/)
assert.match(captureHookSource, /window\.setInterval/)
assert.match(captureHookSource, /filter\(\(message\) => messageReplayTaskId\(message\) === config\.taskId\)/)
assert.match(progressSurfaceSource, /className=\{styles\.progressCommandLineButton\}/)
assert.doesNotMatch(progressSurfaceSource, /<details key=\{`\$\{commandItem\.id\}/)
assert.match(timelineSource, /className=\{styles\.commandRunItemButton\}/)
assert.doesNotMatch(timelineSource, /<details className=\{styles\.commandRunItem\}/)

console.log('pc-frontend task replay tests passed')

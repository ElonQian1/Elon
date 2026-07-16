const assert = require('assert')
const fs = require('fs')
const path = require('path')

const repoRoot = path.resolve(__dirname, '..')
const previewPath = path.join(repoRoot, 'pc-frontend', 'src', 'task-progress-preview.tsx')
const scenarioPreviewPath = path.join(repoRoot, 'pc-frontend', 'src', 'task-progress-preview', 'ScenarioPreview.tsx')
const viteConfigPath = path.join(repoRoot, 'pc-frontend', 'vite.config.ts')
const routerPath = path.join(repoRoot, 'server', 'src', 'router.rs')
const taskGroupPath = path.join(repoRoot, 'pc-frontend', 'src', 'features', 'dev', 'DevTaskGroup.tsx')
const progressSurfacePath = path.join(repoRoot, 'pc-frontend', 'src', 'features', 'dev', 'TaskProgressSurface.tsx')
const timelinePath = path.join(repoRoot, 'pc-frontend', 'src', 'features', 'dev', 'TaskTimeline.tsx')
const taskActionsPath = path.join(repoRoot, 'pc-frontend', 'src', 'features', 'conversation', 'useConversationTaskActions.ts')
const projectStorePath = path.join(repoRoot, 'pc-frontend', 'src', 'features', 'conversation', 'useProjectStore.ts')
const channelAiPath = path.join(repoRoot, 'server', 'src', 'project_space', 'channel_ai.rs')
const conversationTokensPath = path.join(repoRoot, 'pc-frontend', 'src', 'styles', 'conversation-tokens.css')
const replayStylesPath = path.join(repoRoot, 'pc-frontend', 'src', 'task-progress-replay', 'task-progress-replay.css')

const previewSource = fs.readFileSync(previewPath, 'utf8')
const previewUiSource = `${previewSource}\n${fs.readFileSync(scenarioPreviewPath, 'utf8')}`
const viteConfigSource = fs.readFileSync(viteConfigPath, 'utf8')
const routerSource = fs.readFileSync(routerPath, 'utf8')
const taskGroupSource = fs.readFileSync(taskGroupPath, 'utf8')
const progressSurfaceSource = fs.readFileSync(progressSurfacePath, 'utf8')
const timelineSource = fs.readFileSync(timelinePath, 'utf8')
const taskActionsSource = fs.readFileSync(taskActionsPath, 'utf8')
const projectStoreSource = fs.readFileSync(projectStorePath, 'utf8')
const channelAiSource = fs.readFileSync(channelAiPath, 'utf8')
const conversationTokensSource = fs.readFileSync(conversationTokensPath, 'utf8')
const replayStylesSource = fs.readFileSync(replayStylesPath, 'utf8')

assert.match(
  viteConfigSource,
  /taskProgressPreview:\s*resolve\(__dirname,\s*['"]task-progress-preview\.html['"]\)/,
  'the task progress preview must remain a production build entry',
)
assert.strictEqual(
  (routerSource.match(/"\/task-progress-preview\.html"/g) || []).length,
  2,
  'both /pc and /pc-next must serve the preview file before their SPA fallback',
)

const lifecycleScenarios = [
  'queued',
  'first-progress',
  'dispatch',
  'heartbeat',
  'recovery',
  'server-update',
  'win-update',
  'recovery-timeout',
  'thinking',
  'command-running',
  'tool-timeout',
  'tools',
  'tool-failed',
  'approval',
  'resume-required',
  'timeout',
  'done',
  'incomplete',
  'failed',
  'canceled',
  'connection-interrupted',
]

for (const scenario of lifecycleScenarios) {
  assert.match(
    previewSource,
    new RegExp(`id:\\s*['"]${scenario}['"]`),
    `the preview must include the ${scenario} lifecycle scenario`,
  )
}

assert.match(
  previewUiSource,
  /const openProcessInPlace = expandAll/,
  'expand=1 should expose local tool details in every lifecycle scenario',
)
assert.match(previewSource, /收起工具详情/)
assert.match(previewSource, /展开工具详情/)
assert.match(previewSource, /import '\.\/styles\/conversation-tokens\.css'/)
assert.match(conversationTokensSource, /--conversation-reading-width:\s*860px/)
assert.match(conversationTokensSource, /--conversation-body-size:\s*14px/)
assert.match(replayStylesSource, /background:\s*var\(--conversation-canvas\)/)
assert.match(
  taskGroupSource,
  /const directPublicProcess = !resultMsg/,
  'running tasks should render their public flow directly instead of behind a process fold',
)
assert.match(
  taskGroupSource,
  /const publicSurfaceItems = !resultMsg\s*\? progressFlowSurfaceItems/,
  'command and approval events should remain visible even when no assistant note precedes them',
)
assert.match(
  progressSurfaceSource,
  /open=\{expandAll \|\| tone === 'failed'\}/,
  'debug expansion should only open local command details',
)
assert.doesNotMatch(
  timelineSource,
  /title="运行摘要"/,
  'the processed history should not contain a nested runtime summary fold',
)
assert.match(
  taskGroupSource,
  /<TaskTerminalActions[\s\S]*<TaskCompletionMeta timeline=\{timeline\} \/>/,
  'terminal actions and usage metadata should render after the final reply',
)
assert.match(taskGroupSource, /if \(tone === 'canceled'\) return '任务已停止'[\s\S]*return ''/)
assert.doesNotMatch(taskGroupSource, /return '最终回复'/)
assert.match(
  previewUiSource,
  /模拟节点重连/,
  'the recovery preview should exercise the offline-to-online transition',
)
assert.match(
  taskActionsSource,
  /\{ resumeTaskId: taskId \}/,
  'continue must identify the original task instead of sending an unbound prompt',
)
assert.match(projectStoreSource, /resumeTaskId:\s*taskContext\?\.resumeTaskId/)
assert.match(channelAiSource, /alias = "resumeTaskId"/)
assert.doesNotMatch(
  taskActionsSource,
  /继续处理这个任务。/,
  'continue should not rely on an ambiguous natural-language retry message',
)
assert.match(
  progressSurfaceSource,
  /item\.kind === 'approval'.*taskContext/s,
  'tool approval must remain actionable in the direct progress flow',
)
assert.match(
  progressSurfaceSource,
  /命令失败后，AI 正在根据报错定位并修复/,
  'an active command failure should explain that AI remains in control',
)
assert.match(
  previewUiSource,
  /window\.history\.replaceState/,
  'scenario and expansion controls should update the shareable preview URL',
)
assert.match(previewSource, /updatePreviewLocation\(view, undefined\)/)
assert.match(previewSource, /updatePreviewLocation\(undefined, next\)/)
assert.match(previewSource, /url\.searchParams\.set\('view', view\)/)
assert.match(previewSource, /url\.searchParams\.set\('expand', expand \? '1' : '0'\)/)

console.log('pc-frontend task-progress preview tests passed')

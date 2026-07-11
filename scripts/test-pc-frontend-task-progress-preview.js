const assert = require('assert')
const fs = require('fs')
const path = require('path')

const repoRoot = path.resolve(__dirname, '..')
const previewPath = path.join(repoRoot, 'pc-frontend', 'src', 'task-progress-preview.tsx')
const viteConfigPath = path.join(repoRoot, 'pc-frontend', 'vite.config.ts')
const routerPath = path.join(repoRoot, 'server', 'src', 'router.rs')
const taskGroupPath = path.join(repoRoot, 'pc-frontend', 'src', 'features', 'dev', 'DevTaskGroup.tsx')
const progressSurfacePath = path.join(repoRoot, 'pc-frontend', 'src', 'features', 'dev', 'TaskProgressSurface.tsx')
const timelinePath = path.join(repoRoot, 'pc-frontend', 'src', 'features', 'dev', 'TaskTimeline.tsx')

const previewSource = fs.readFileSync(previewPath, 'utf8')
const viteConfigSource = fs.readFileSync(viteConfigPath, 'utf8')
const routerSource = fs.readFileSync(routerPath, 'utf8')
const taskGroupSource = fs.readFileSync(taskGroupPath, 'utf8')
const progressSurfaceSource = fs.readFileSync(progressSurfacePath, 'utf8')
const timelineSource = fs.readFileSync(timelinePath, 'utf8')

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
]

for (const scenario of lifecycleScenarios) {
  assert.match(
    previewSource,
    new RegExp(`id:\\s*['"]${scenario}['"]`),
    `the preview must include the ${scenario} lifecycle scenario`,
  )
}

assert.match(
  previewSource,
  /const openProcessInPlace = expandAll/,
  'expand=1 should expose local tool details in every lifecycle scenario',
)
assert.match(previewSource, /收起工具详情/)
assert.match(previewSource, /展开工具详情/)
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
  /afterBubble=\{<TaskCompletionMeta timeline=\{timeline\} \/>\}/,
  'usage metadata should render after the final reply',
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
  previewSource,
  /window\.history\.replaceState/,
  'scenario and expansion controls should update the shareable preview URL',
)
assert.match(previewSource, /updatePreviewLocation\(view, undefined\)/)
assert.match(previewSource, /updatePreviewLocation\(undefined, next\)/)
assert.match(previewSource, /url\.searchParams\.set\('view', view\)/)
assert.match(previewSource, /url\.searchParams\.set\('expand', expand \? '1' : '0'\)/)

console.log('pc-frontend task-progress preview tests passed')

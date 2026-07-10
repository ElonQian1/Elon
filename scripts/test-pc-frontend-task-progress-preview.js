const assert = require('assert')
const fs = require('fs')
const path = require('path')

const repoRoot = path.resolve(__dirname, '..')
const previewPath = path.join(repoRoot, 'pc-frontend', 'src', 'task-progress-preview.tsx')
const viteConfigPath = path.join(repoRoot, 'pc-frontend', 'vite.config.ts')
const routerPath = path.join(repoRoot, 'server', 'src', 'router.rs')

const previewSource = fs.readFileSync(previewPath, 'utf8')
const viteConfigSource = fs.readFileSync(viteConfigPath, 'utf8')
const routerSource = fs.readFileSync(routerPath, 'utf8')

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
  'expand=1 should expose the process in every lifecycle scenario',
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

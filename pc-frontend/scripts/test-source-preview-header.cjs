const assert = require('node:assert/strict')
const fs = require('node:fs')
const path = require('node:path')

const projectRoot = path.resolve(__dirname, '..')
const read = (file) => fs.readFileSync(path.join(projectRoot, file), 'utf8')
const rule = (css, className) => (
  css.match(new RegExp(`\\.${className}\\s*\\{[^}]*\\}`))?.[0] ?? ''
)

const modeBarSource = read('src/features/ui-tuner/source-preview/SourcePreviewModeBar.tsx')
const previewSource = read('src/features/ui-tuner/source-preview/PwaInteractivePreviewSurface.tsx')
const previewCss = read('src/features/ui-tuner/source-preview/SourcePreview.module.css')
const workspaceModeSource = read('src/features/ui-tuner/workspace/UiWorkspaceModeBar.tsx')
const progressSource = read('src/features/ui-tuner/workspace/UiDesignProgressBar.tsx')
const progressCss = read('src/features/ui-tuner/workspace/UiDesignProgressBar.module.css')

assert.match(modeBarSource, /styles\.modeBarContextRow/)
assert.match(modeBarSource, /styles\.modeBarActionRow/)
assert.match(modeBarSource, /title=\{props\.projectRoot\}/)
assert.match(modeBarSource, /aria-label="当前布局文件"/)
assert.doesNotMatch(rule(previewCss, 'modeBar'), /overflow\s*:\s*hidden/)
assert.match(previewCss, /@media\(max-width:1000px\)\{\.projectContext,\.rendererControls,\.editActions/)

assert.match(workspaceModeSource, /\{!compact && <>/)
assert.match(progressSource, /<UiDesignProgressBar compact steps=/)
assert.match(progressSource, /aria-current=\{step\.state === 'active' \? 'step'/)
assert.match(rule(progressCss, 'progress'), /minmax\(0,\s*1fr\)/)
assert.match(progressCss, /\.progress li\s*\{[^}]*min-width:\s*0/s)

assert.match(
  previewSource,
  /className=\{styles\.pwaPreviewToolbar\}[\s\S]*?className=\{styles\.pwaWorkflowGuide\}/,
)
assert.match(previewSource, /步骤 \{modeStep\(design\)\}\/4/)
assert.doesNotMatch(previewSource, /styles\.pwaModeGuide/)
assert.match(rule(previewCss, 'pwaPreviewToolbar'), /grid-template-columns/)
assert.match(rule(previewCss, 'pwaWorkflowGuide'), /min-width:\s*0/)
assert.match(rule(previewCss, 'pwaDeviceToolbar'), /display:\s*flex/)
assert.match(previewCss, /\.pwaDeviceAdvanced\[open\]\{[^}]*flex-basis:100%/)

console.log('source preview header: all assertions passed')

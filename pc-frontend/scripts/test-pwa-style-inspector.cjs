const assert = require('node:assert/strict')
const fs = require('node:fs')
const path = require('node:path')

const projectRoot = path.resolve(__dirname, '..')
const sourceRoot = path.join(projectRoot, 'src/features/ui-tuner/source-preview')
const read = (file) => fs.readFileSync(path.join(sourceRoot, file), 'utf8')

const inspectorSource = read('PwaStyleInspector.tsx')
assert.match(inspectorSource, /样式属性/)
assert.match(inspectorSource, /className=\{panelStyles\.inspectorChrome\}/)
assert.match(inspectorSource, /<PwaStyleEditor session=\{session\}/)
assert.match(inspectorSource, /工作流与交付/)
assert.match(inspectorSource, /panelStyles\.resetActions/)

const editorSource = read('PwaStyleEditor.tsx')
for (const section of ['尺寸与布局', '间距', '形状与文字', '颜色与外观']) {
  assert.match(editorSource, new RegExp(`title="${section}"`))
}
assert.match(editorSource, /内边距/)
assert.match(editorSource, /外边距/)
assert.match(editorSource, /颜色自身透明度请在颜色面板中调整/)
assert.match(editorSource, /changedCount\(session/)

const fieldsSource = read('PwaStyleFields.tsx')
assert.match(fieldsSource, /按住 Shift 调整 10 倍步长/)
assert.match(fieldsSource, /event\.key !== 'ArrowUp'/)
assert.match(fieldsSource, /event\.shiftKey \? 10 : 1/)
assert.match(fieldsSource, /className=\{styles\.stepper\}/)

const inspectorCss = read('PwaStyleInspector.module.css')
assert.match(inspectorCss, /\.inspectorChrome\s*\{[^}]*position:\s*sticky/s)
assert.match(inspectorCss, /\.edgeGrid\s*\{[^}]*grid-template-columns:\s*repeat\(2/s)
assert.match(inspectorCss, /\.workflowDisclosure/)
assert.match(inspectorCss, /\.resetActions\s*\{[^}]*position:\s*sticky/s)
assert.match(inspectorCss, /@media \(max-width: 1050px\)/)

console.log('PWA_STYLE_INSPECTOR_TEST=passed')

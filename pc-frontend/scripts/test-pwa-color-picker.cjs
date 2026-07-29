const assert = require('node:assert/strict')
const fs = require('node:fs')
const path = require('node:path')

const projectRoot = path.resolve(__dirname, '..')
const sourceRoot = path.join(projectRoot, 'src/features/ui-tuner/source-preview')
const read = (file) => fs.readFileSync(path.join(sourceRoot, file), 'utf8')

const fieldSource = read('PwaColorStyleField.tsx')
assert.match(fieldSource, /lazy\(\(\) => import\('\.\/PwaColorStyleFieldControl'\)\)/)

const controlSource = read('PwaColorStyleFieldControl.tsx')
assert.match(controlSource, /lazy\(\(\) => import\('\.\/PwaColorPopover'\)\)/)
assert.match(controlSource, /aria-haspopup="dialog"/)
assert.match(controlSource, /event\.key === 'Escape'/)
assert.match(controlSource, /spaceBelow < 360/)
assert.match(controlSource, /placeholder="#222255 \/ rgba\(34,34,85,\.9\)"/)

const popoverSource = read('PwaColorPopover.tsx')
assert.match(popoverSource, /EyeDropper/)
assert.match(popoverSource, /pwa-color-saturation-value/)
assert.match(popoverSource, /className=\{styles\.hueSlider\}/)
assert.match(popoverSource, /className=\{styles\.alphaSlider\}/)
assert.match(popoverSource, /<option value="hex">HEX<\/option>/)
assert.match(popoverSource, /PRESET_COLORS\.map/)

const pickerCss = read('PwaColorPicker.module.css')
assert.match(pickerCss, /\.saturationPlane/)
assert.match(pickerCss, /\.alphaSlider/)
assert.match(pickerCss, /\.palette/)
assert.match(pickerCss, /\.popover\[data-placement="above"\]/)

const inspectorSource = read('PwaStyleInspector.tsx')
assert.match(inspectorSource, /<span>整体透明度<\/span>/)

console.log('PWA_COLOR_PICKER_TEST=passed')

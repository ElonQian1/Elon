const assert = require('node:assert/strict')
const fs = require('node:fs')
const path = require('node:path')

const projectRoot = path.resolve(__dirname, '..')
const read = (relative) => fs.readFileSync(path.join(projectRoot, relative), 'utf8')

const api = read('src/features/ui-tuner/headless-design/designPlanningApi.ts')
const controls = read('src/features/ui-tuner/headless-design/useDesignPlanningControls.ts')
const review = read('src/features/ui-tuner/headless-design/DesignPlanningReview.tsx')
const context = read('src/features/ui-tuner/contextPack.ts')

assert.match(api, /comparisons\/\$\{encodeURIComponent\(comparisonId\)\}\/run/)
assert.match(controls, /runDesignRegressionComparison/)
assert.match(review, /运行本机比较/)
assert.match(context, /ui_run_design_regression_comparison/)

console.log('headless design local regression comparator contract passed')

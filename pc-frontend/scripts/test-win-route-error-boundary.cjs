const assert = require('node:assert/strict')
const fs = require('node:fs')
const path = require('node:path')

const read = (relative) => fs.readFileSync(path.resolve(__dirname, '..', relative), 'utf8')
const main = read('src/main.tsx')
const boundary = read('src/WorkbenchErrorBoundary.tsx')

assert.match(main, /<WorkbenchErrorBoundary>/)
assert.match(main, /<BrowserRouter basename="\/pc">/)
assert.match(boundary, /static getDerivedStateFromError/)
assert.match(boundary, /componentDidCatch/)
assert.match(boundary, /已阻止整窗黑屏/)
assert.match(boundary, /window\.location\.reload\(\)/)
assert.match(boundary, /window\.location\.assign\(window\.location\.pathname\.startsWith\('\/pc'\) \? '\/pc\/local-tasks' : '\/'\)/)

process.stdout.write('PASS Win route error boundary contract\n')

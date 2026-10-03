// Child WebView geometry belongs to webview_layout.rs. Do not read stale Wry
// coordinates through a partial set_position/set_size update after reparent.
const fs = require('node:fs')
const path = require('node:path')
const { execFileSync } = require('node:child_process')
const assert = require('node:assert/strict')
const root = path.resolve(__dirname, '..')
const sourceRoot = 'desktop-shell/src-tauri/src/'
const owner = `${sourceRoot}webview_layout.rs`

function violations(file, source) {
  const code = source.replace(/\/\*[\s\S]*?\*\//g, '').replace(/\/\/[^\n]*/g, '')
  return [...code.matchAll(/\.\s*(set_position|set_size|set_bounds)\s*\(/g)]
    .filter(match => file !== owner || match[1] !== 'set_bounds')
    .map(match => `${file}: ${match[1]} must use webview_layout`)
}

function files(directory) {
  return fs.readdirSync(directory, { withFileTypes: true }).flatMap(entry => {
    const full = path.join(directory, entry.name)
    return entry.isDirectory() ? files(full) : full.endsWith('.rs') ? [full] : []
  })
}

assert.equal(violations('new_view.rs', 'page.set_size(size);').length, 1)
assert.equal(violations('new_view.rs', 'page\n .set_position(pos);').length, 1)
assert.equal(violations('new_view.rs', 'page.set_bounds(rect);').length, 1)
assert.equal(violations(owner, 'page.set_position(pos);').length, 1)
assert.equal(violations(owner, 'page.set_bounds(rect);').length, 0)
assert.equal(violations('new_view.rs', '// page.set_size(size);\nwebview_layout::place(&page, pos, size);').length, 0)

const staged = process.argv.includes('--staged')
const changed = staged
  ? execFileSync('git', ['diff', '--cached', '--name-only', '--diff-filter=ACMR', '-z'], { cwd: root, encoding: 'utf8' })
      .split('\0').filter(file => file.startsWith(sourceRoot) && file.endsWith('.rs'))
  : files(path.join(root, sourceRoot)).map(file => path.relative(root, file).replaceAll('\\', '/'))
const errors = changed.flatMap(file => violations(file, staged
  ? execFileSync('git', ['show', `:${file}`], { cwd: root, encoding: 'utf8' })
  : fs.readFileSync(path.join(root, file), 'utf8')))
if (errors.length) {
  console.error(errors.join('\n'))
  process.exit(1)
}
console.log(`WEBVIEW_LAYOUT_GUARD=passed files=${changed.length} staged=${staged}`)

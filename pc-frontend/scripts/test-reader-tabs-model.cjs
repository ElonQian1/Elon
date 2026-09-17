// Pure reading-tab transitions: open/dedupe, Chrome-like close fallback, layout, width clamp.
const assert = require('node:assert/strict')
const fs = require('node:fs')
const path = require('node:path')
const vm = require('node:vm')
const ts = require('typescript')
const source = fs.readFileSync(path.join(__dirname, '../src/features/reader/readerTabsModel.ts'), 'utf8')
const exportsTs = {}
vm.runInNewContext(ts.transpileModule(source, { compilerOptions: { module: ts.ModuleKind.CommonJS } }).outputText, { exports: exportsTs, require })
const m = exportsTs
const preview = url => ({ schema: 1, url, title: '', site: 'X', author: '', description: '', image: null, embed: null, status: 'unavailable', source: 'server' })
const tab = (id, url) => ({ id, url, originalUrl: url, title: 't', site: 'X', preview: preview(url), scope: 's' })

let state = m.initialReaderTabsState
assert.equal(state.layout, 'docked'); assert.equal(state.presented, false)
state = m.openTab(state, tab('a', 'https://x.com/a/status/1'))
state = m.openTab(state, tab('b', 'https://x.com/a/status/2'))
state = m.openTab(state, tab('c', 'https://x.com/a/status/3'))
const ids = s => s.tabs.map(t => t.id).join(',')
assert.equal(ids(state), 'a,b,c'); assert.equal(state.activeId, 'c'); assert.equal(state.presented, true)
// Reopening the same original link re-activates instead of duplicating.
state = m.openTab(state, tab('dup', 'https://x.com/a/status/1'))
assert.equal(state.tabs.length, 3); assert.equal(state.activeId, 'a')
// Closing the active tab prefers the right neighbour, then the left one.
state = m.activateTab(state, 'b'); state = m.closeTab(state, 'b')
assert.equal(ids(state), 'a,c'); assert.equal(state.activeId, 'c')
state = m.closeTab(state, 'c'); assert.equal(state.activeId, 'a')
state = m.closeTab(state, 'a'); assert.equal(state.activeId, null); assert.equal(state.presented, false)
assert.equal(m.closeTab(state, 'missing'), state)
// Layout, presentation and patches.
state = m.openTab(state, tab('a', 'https://x.com/a/status/1'))
state = m.setPresented(state, false); assert.equal(state.presented, false)
state = m.setLayout(state, 'overlay'); assert.equal(state.layout, 'overlay'); assert.equal(state.presented, true)
state = m.updateTab(state, 'a', { hosted: 'popout', title: 'T' })
assert.equal(state.tabs[0].hosted, 'popout'); assert.equal(state.tabs[0].title, 'T')
assert.equal(m.updateTab(state, 'zzz', { title: 'x' }), state)
// Capacity and width clamp.
let full = m.initialReaderTabsState
for (let i = 0; i < 12; i++) full = m.openTab(full, tab('t' + i, 'https://x.com/a/status/' + (100 + i)))
assert.equal(full.tabs.length, m.MAX_READER_TABS)
assert.equal(m.clampDockWidth(100, 1600), m.MIN_DOCK_WIDTH)
assert.equal(m.clampDockWidth(5000, 1600), Math.floor(1600 * m.MAX_DOCK_WIDTH_RATIO))
assert.equal(m.clampDockWidth(500.4, 1600), 500)
assert.equal(m.badgeFor('微信公众号'), '文'); assert.equal(m.badgeFor('unknown'), '↗')
assert.notEqual(m.nextTabId(1), m.nextTabId(1))
console.log('PASS: reader tabs model — dedupe, neighbour fallback, layout/presented, capacity, width clamp')

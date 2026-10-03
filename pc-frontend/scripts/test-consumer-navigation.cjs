const assert = require('node:assert/strict')
const fs = require('node:fs')
const path = require('node:path')
const vm = require('node:vm')
const ts = require('typescript')

const root = path.resolve(__dirname, '..')
const jsx = (type, props) => ({ type, props: props || {} })
const icons = new Proxy({}, { get: (_, name) => `icon-${String(name)}` })
const sameDeps = (left, right) => left && right && left.length === right.length
  && left.every((value, index) => Object.is(value, right[index]))

function createHooks() {
  const slots = []
  let cursor = 0
  let pending = []
  return {
    api: {
      useState(initial) {
        const index = cursor++
        slots[index] ??= { value: typeof initial === 'function' ? initial() : initial }
        return [slots[index].value, (next) => {
          slots[index].value = typeof next === 'function' ? next(slots[index].value) : next
        }]
      },
      useCallback(callback, deps) {
        const index = cursor++
        if (!sameDeps(slots[index]?.deps, deps)) slots[index] = { value: callback, deps }
        return slots[index].value
      },
      useEffect(effect, deps) {
        const index = cursor++
        if (!sameDeps(slots[index]?.deps, deps)) {
          pending.push(() => {
            slots[index]?.cleanup?.()
            slots[index] = { deps, cleanup: effect() }
          })
        }
      },
    },
    render(component) {
      cursor = 0
      pending = []
      const tree = component()
      pending.forEach((effect) => effect())
      return tree
    },
  }
}

function load(relative, mocks, globals = {}) {
  const filename = path.join(root, relative)
  const output = ts.transpileModule(fs.readFileSync(filename, 'utf8'), {
    fileName: filename,
    compilerOptions: { module: ts.ModuleKind.CommonJS, target: ts.ScriptTarget.ES2022, jsx: ts.JsxEmit.ReactJSX },
  }).outputText
  const module = { exports: {} }
  vm.runInNewContext(output, {
    module, exports: module.exports, URL, URLSearchParams, Intl, console, ...globals,
    require(request) {
      if (request === 'react/jsx-runtime') return { jsx, jsxs: jsx, Fragment: 'fragment' }
      if (request === 'lucide-react') return icons
      if (request.endsWith('.module.css')) return new Proxy({}, { get: (_, key) => key })
      if (Object.hasOwn(mocks, request)) return mocks[request]
      throw new Error(`Unexpected import ${request} in ${relative}`)
    },
  }, { filename })
  return module.exports
}

function elements(tree) {
  if (!tree || typeof tree !== 'object') return []
  if (Array.isArray(tree)) return tree.flatMap(elements)
  return [tree, ...elements(tree.props?.children)]
}

function byLabel(tree, label) {
  return elements(tree).find((element) => element.type === 'button' && element.props['aria-label'] === label)
}

function verifyNavigation() {
  const model = load('src/features/shell/navigationModel.ts', {})
  const developerPaths = ['/git-worktrees', '/codex-control', '/browser-research', '/local-tasks']
  const development = model.sectionsForWorkspace('projects').find((section) => section.label === '开发工具')
  assert.ok(development, 'project navigation exposes a discoverable development group')
  for (const destination of developerPaths) {
    assert.ok(development.items.some((item) => item.path === destination), `${destination} remains accessible`)
    assert.equal(model.workspaceForPath(destination), 'projects')
    assert.ok(!model.WORKSPACE_RAIL_ITEMS.some((item) => item.path === destination), 'developer tools stay out of the primary rail')
  }

  for (const mode of ['local', 'cloud']) {
    for (const cloudState of ['checking', 'online', 'offline']) {
      for (const role of ['member', 'admin']) {
        const hooks = createHooks()
        const destinations = []
        const presenceRequests = []
        const user = { id: 'consumer', nickname: '消费者', role }
        const component = load('src/features/shell/ServerRail.tsx', {
          react: hooks.api,
          'react-router-dom': { useLocation: () => ({ pathname: '/ai' }), useNavigate: () => (to) => destinations.push(to) },
          '../../store/auth': { useAuthStore: (selector) => selector({ user }) },
          '../conversation/useProjectStore': { useProjectStore: (selector) => selector({ projects: [], activeProjectId: '' }) },
          './UserAvatar': { default: 'UserAvatar', userDisplayName: (value) => value.nickname },
          './useMyPresence': { presenceLabel: () => '在线', useMyPresence: (enabled) => { presenceRequests.push(enabled); return null } },
          './useWorkbenchConnection': { useWorkbenchConnection: () => ({ cloudState }) },
          './navigationModel': model,
        }, { window: { __ELON_PC_BOOTSTRAP__: { mode } } }).default
        const tree = hooks.render(component)
        for (const [label, destination] of [['AI 工作区', '/ai'], ['项目', '/projects'], ['消息', '/friends'], ['算力', '/compute-market'], ['连接与设备', '/node']]) {
          const button = byLabel(tree, label)
          assert.ok(button, `${mode}/${cloudState}: ${label} remains visible`)
          button.props.onClick()
          assert.equal(destinations.at(-1), destination)
        }
        assert.equal(Boolean(byLabel(tree, '管理')), role === 'admin')
        assert.equal(byLabel(tree, '本机任务'), undefined)
        assert.equal(byLabel(tree, 'Codex 控制台'), undefined)
        assert.equal(presenceRequests.at(-1), cloudState !== 'offline')
        const connection = byLabel(tree, '连接与设备')
        assert.equal(connection.props['data-cloud-state'], cloudState)
        assert.match(connection.props.title, /云端/)
        assert.doesNotMatch(connection.props.title, /节点正常|节点已连接/)
      }
    }
  }
}

async function verifyDiagnostics() {
  const hooks = createHooks()
  const reads = { tasks: 0, evolution: 0, publish: 0 }
  const timers = new Map()
  let timerId = 0
  const location = { href: 'http://127.0.0.1:7799/pc/local-tasks', search: '' }
  const component = load('src/features/local-tasks/LocalTasksPage.tsx', {
    react: hooks.api,
    '../../api/runtime': { isLocalWorkbench: () => true },
    '../../lib/utils': { safeNodeAdminUrl: () => '' },
    '../../lib/taskTitle': { readableTaskTitle: (value) => value },
    '../conversation/localPcRuntime': {},
    './LocalTaskCreateForm': { default: 'LocalTaskCreateForm' },
    './LocalTaskDetailPanel': { default: 'LocalTaskDetailPanel' },
    './LocalOperationsPanel': { default: 'LocalOperationsPanel' },
    './localTaskApi': {
      listLocalTasks: async () => { reads.tasks += 1; return { tasks: [] } },
      listSelfEvolution: async () => { reads.evolution += 1; return {} },
      getGlobalPublishStatus: async () => { reads.publish += 1; return {} },
    },
    './localOperationsModel': { normalizeGlobalPublishStatus: (value) => value, normalizeSelfEvolutionQueue: (value) => value },
    './localTaskModel': { normalizeLocalTaskList: (value) => value.tasks, pendingSyncCountFromList: () => 0 },
  }, {
    location,
    window: {
      history: { state: {}, replaceState() {} },
      setInterval(callback) { timers.set(++timerId, callback); return timerId },
      clearInterval(id) { timers.delete(id) },
    },
  }).default
  let tree = hooks.render(component)
  let details = elements(tree).find((element) => element.type === 'details')
  assert.ok(details, 'advanced diagnostics has a native disclosure control')
  assert.ok(!details.props.open, 'diagnostics starts closed')
  assert.ok(!elements(tree).some((element) => element.type === 'LocalOperationsPanel'))
  assert.deepEqual(reads, { tasks: 1, evolution: 0, publish: 0 }, 'opening local tasks does not poll management data')
  assert.equal(timers.size, 1, 'ordinary task refresh remains active')

  details.props.onToggle({ currentTarget: { open: true } })
  tree = hooks.render(component)
  assert.ok(elements(tree).some((element) => element.type === 'LocalOperationsPanel' && typeof element.props.onAction === 'function'))
  assert.deepEqual(reads, { tasks: 1, evolution: 1, publish: 1 }, 'expanding diagnostics immediately refreshes both panels')
  assert.equal(timers.size, 2)
  await new Promise((resolve) => setImmediate(resolve))

  details = elements(tree).find((element) => element.type === 'details')
  details.props.onToggle({ currentTarget: { open: false } })
  tree = hooks.render(component)
  assert.ok(!elements(tree).some((element) => element.type === 'LocalOperationsPanel'))
  assert.equal(timers.size, 1, 'closing diagnostics cancels its polling')
  for (const callback of timers.values()) callback()
  assert.deepEqual(reads, { tasks: 2, evolution: 1, publish: 1 }, 'task refresh continues without management requests')
}

async function main() {
  verifyNavigation()
  await verifyDiagnostics()
  console.log('PASS consumer navigation: 12 runtime/connection/role cases; diagnostics request and disclosure lifecycle')
}

main().catch((error) => { console.error(error); process.exitCode = 1 })

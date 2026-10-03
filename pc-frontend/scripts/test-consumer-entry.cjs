const assert = require('node:assert/strict')
const fs = require('node:fs')
const path = require('node:path')
const vm = require('node:vm')
const ts = require('typescript')
const React = require('react')
const { renderToStaticMarkup } = require('react-dom/server')
const router = require('react-router-dom')

const root = path.resolve(__dirname, '..')
const noop = () => null
const componentStub = new Proxy({ __esModule: true, default: noop }, {
  get: (target, key) => key in target ? target[key] : noop,
})
const cssStub = { __esModule: true, default: new Proxy({}, { get: (_, key) => String(key) }) }

function environment(href, mode, entries = {}) {
  const values = new Map(Object.entries(entries))
  const localStorage = {
    getItem: key => values.get(key) ?? null,
    setItem: (key, value) => values.set(key, String(value)),
    removeItem: key => values.delete(key),
  }
  const assigned = []
  const location = Object.assign(new URL(href), { assign: value => assigned.push(value), reload: noop })
  const window = { location, localStorage, dispatchEvent: noop,
    __ELON_PC_BOOTSTRAP__: mode === 'local'
      ? { mode, cloudBaseUrl: 'https://cloud.example', localNodeBaseUrl: location.origin }
      : undefined }
  return { window, location, localStorage, assigned, URL, URLSearchParams,
    console, setTimeout, clearTimeout, AbortController }
}

function load(relative, globals, dependencies = {}, fallback) {
  const filename = path.join(root, relative)
  const source = fs.readFileSync(filename, 'utf8')
    .replaceAll('import.meta.env', '({ DEV: false, BASE_URL: "/pc/" })')
  const compiled = ts.transpileModule(source, { fileName: filename, compilerOptions: {
    module: ts.ModuleKind.CommonJS, target: ts.ScriptTarget.ES2022, jsx: ts.JsxEmit.ReactJSX,
  } }).outputText
  const module = { exports: {} }
  const localRequire = id => {
    if (Object.hasOwn(dependencies, id)) return dependencies[id]
    if (id === 'react' || id === 'react/jsx-runtime') return require(id)
    if (id.endsWith('.css')) return cssStub
    if (fallback) return fallback(id)
    throw new Error(`Unexpected dependency in ${relative}: ${id}`)
  }
  vm.runInNewContext(compiled, { ...globals, exports: module.exports, module, require: localRequire }, { filename })
  return module.exports
}

function descendants(element) {
  if (!React.isValidElement(element)) return []
  return [element, ...React.Children.toArray(element.props.children).flatMap(descendants)]
}

function hookHarness() {
  const states = []
  let cursor = 0
  return {
    react: { ...React, useEffect: noop, useMemo: fn => fn(), useCallback: fn => fn,
      useRef: value => ({ current: value }),
      useState: initial => {
        const index = cursor++
        if (!(index in states)) states[index] = typeof initial === 'function' ? initial() : initial
        return [states[index], next => { states[index] = typeof next === 'function' ? next(states[index]) : next }]
      } },
    render: fn => { cursor = 0; return fn() },
  }
}

function entryModules(globals) {
  const runtime = load('src/api/runtime.ts', globals)
  const entry = load('src/features/shell/workbenchEntry.ts', globals, {
    '../../api/runtime': runtime,
  })
  return { runtime, entry }
}

function testRoutesAndTransport() {
  for (const [href, mode] of [
    ['http://127.0.0.1:7799/pc', 'local'],
    ['http://127.0.0.1:7803/pc', 'local'],
    ['https://cloud.example/pc', 'cloud'],
    ['https://cloud.example/pc?node_admin=http%3A%2F%2F127.0.0.1%3A7803%2F', 'cloud'],
  ]) {
    const env = environment(href, mode)
    const { runtime, entry } = entryModules(env)
    const App = load('src/App.tsx', env, {
      'react-router-dom': { ...router, useLocation: () => env.location }, './api/runtime': runtime,
      './features/shell/workbenchEntry': entry,
    }, () => componentStub).default
    const routeTree = router.createRoutesFromElements(App().props.children)
    const rootMatch = router.matchRoutes(routeTree, '/').at(-1).route
    assert.equal(rootMatch.element.type, router.Navigate)
    const target = rootMatch.element.props.to
    assert.equal(typeof target === 'string' ? target.split('?')[0] : target.pathname, '/ai', `${mode} root opens the consumer workbench`)
    if (env.location.search) assert.equal(new URL(target, href).searchParams.get('node_admin'), 'http://127.0.0.1:7803/')
    assert.equal(router.matchRoutes(routeTree, '/ai').at(-1).route.path, 'ai')
    const tasksRoute = router.matchRoutes(routeTree, '/local-tasks').at(-1).route
    assert.equal(tasksRoute.path, 'local-tasks', 'an explicit local task URL stays available')
    assert.notEqual(tasksRoute.element.type, router.Navigate, 'explicit task navigation is not replaced by the home redirect')

    assert.equal(runtime.isLocallyHostedWorkbench(), mode === 'local')
    assert.equal(runtime.isLocalWorkbench(), mode === 'local', 'legacy transport callers keep their hosting detection')
    assert.equal(runtime.resolveApiUrl('/api/me'), mode === 'local' ? 'https://cloud.example/api/me' : '/api/me')
    assert.equal(runtime.cloudWebSocketUrl('/ws/app'), 'wss://cloud.example/ws/app')
    assert.equal(runtime.resolveApiUrl('https://other.example/api/public'), 'https://other.example/api/public')
    const calls = []
    const localTasks = load('src/features/local-tasks/localTaskApi.ts', env, {
      '../../lib/utils': { safeNodeAdminUrl: () => runtime.localNodeBaseUrl() },
      '../node/localNodeApi': { nodeApi: (...args) => { calls.push(args); return Promise.resolve({}) } },
      '../../api/client': { api: { get: () => { throw new Error('Local tasks must not use cloud API transport') } } },
    })
    localTasks.listLocalTasks()
    assert.equal(calls[0][0], runtime.localNodeBaseUrl())
    assert.equal(new URL(calls[0][1], calls[0][0]).hostname, '127.0.0.1')
    assert.equal(new URL(calls[0][1], calls[0][0]).pathname, '/api/local-tasks')
  }
}

function testSafeRecovery() {
  for (const nodeAdmin of ['http://127.0.0.1:7803/', 'http://localhost:7799/', 'https://outside.example/']) {
    const env = environment(`http://127.0.0.1:7799/pc/local-tasks?node_admin=${encodeURIComponent(nodeAdmin)}&token=do-not-forward`, 'local')
    const { entry } = entryModules(env)
    assert.equal(entry.WORKBENCH_HOME_ROUTE, '/ai')
    const Boundary = load('src/WorkbenchErrorBoundary.tsx', env, {
      './features/shell/workbenchEntry': entry,
    }).default
    const boundary = new Boundary({ children: React.createElement('p', null, 'content') })
    assert.match(renderToStaticMarkup(boundary.render()), /content/)
    boundary.state = Boundary.getDerivedStateFromError(new Error('fixture failure'))
    const buttons = descendants(boundary.render()).filter(element => element.type === 'button')
    const homeButton = buttons.find(element => /返回.*工作台/.test(String(element.props.children)))
    assert.ok(homeButton, 'the render failure offers the consumer workbench as a recovery destination')
    homeButton.props.onClick()
    const destination = new URL(env.assigned.at(-1), env.location.href)
    assert.equal(destination.pathname, '/pc/ai')
    assert.equal(destination.searchParams.has('token'), false)
    assert.equal(destination.searchParams.get('node_admin'), nodeAdmin.includes('outside') ? null : nodeAdmin)
  }
}

async function testLogin() {
  for (const mode of ['login', 'register']) {
    const env = environment(`http://127.0.0.1:7799/pc/login?mode=${mode}&node_admin=http%3A%2F%2F127.0.0.1%3A7803%2F`, 'local')
    const { entry } = entryModules(env)
    const hooks = hookHarness()
    const navigations = [], sessions = []
    const auth = {
      login: async (...args) => sessions.push(['login', ...args]),
      register: async (...args) => sessions.push(['register', ...args]),
      acceptSession: noop,
    }
    const Login = load('src/features/auth/LoginPage.tsx', env, {
      react: hooks.react,
      'react-router-dom': { useNavigate: () => (...args) => navigations.push(args), useSearchParams: () => [new URLSearchParams(env.location.search)] },
      '../../store/auth': { useAuthStore: selector => selector(auth) },
      '../shell/workbenchEntry': entry,
      '../shell/pcLegacyUrl': { getPcLegacyUrl: () => '/pc-legacy/' },
    }, () => componentStub).default
    let tree = hooks.render(Login)
    for (const input of descendants(tree).filter(element => element.type === 'input')) {
      input.props.onChange({ target: { value: input.props.autoComplete === 'username' ? 'fixture-user' : 'fixture-password' } })
    }
    tree = hooks.render(Login)
    await descendants(tree).find(element => element.type === 'form').props.onSubmit({ preventDefault: noop })
    assert.equal(sessions[0][0], mode)
    assert.equal(navigations.length, 1)
    const destination = new URL(navigations[0][0], env.location.href)
    assert.equal(destination.pathname, '/ai', `${mode} success returns to consumer AI`)
    assert.equal(destination.searchParams.get('node_admin'), 'http://127.0.0.1:7803/')
    assert.equal(navigations[0][1].replace, true)
  }
}

function testUserPanelPreference() {
  for (const preference of [undefined, 'false', 'true']) {
    const values = preference === undefined ? {} : { 'elon.pc.aiUserPanelCollapsed': preference }
    const env = environment('http://127.0.0.1:7799/pc/ai', 'local', values)
    const hooks = hookHarness()
    const modelCopy = { title: 'Fixture model', source: 'AI', detail: 'Fixture' }
    const AiChatPage = load('src/features/ai/AiChatPage.tsx', env, {
      react: hooks.react, 'react-dom': { createPortal: noop },
      'react-router-dom': { useNavigate: () => noop },
      uuid: { v4: () => 'fixture-conversation' },
      '../../store/auth': { useAuthStore: selector => selector({ user: null }) },
      '../models/useModelStore': { useModelStore: selector => selector({ selectedAgent: '', label: '', options: [] }) },
      '../models/routeModelPolicy': { routeModelButtonCopy: () => modelCopy },
      '../conversation/runtimeRoutes': { initialRuntimeRouteFromStorage: () => 'route_c3' },
      '../user-browser/useLocalAiOwnerIdentity': { __esModule: true, default: () => ({ ownerKey: 'anonymous-device:fixture' }) },
      '../user-browser/useAiWebChatBackend': { __esModule: true, default: () => ({}) },
    }, () => componentStub).default
    const tree = hooks.render(() => AiChatPage({ mode: 'work', onModeChange: noop }))
    const markup = renderToStaticMarkup(tree)
    assert.equal(markup.includes('全站用户'), preference !== 'true', 'new users see the panel; an explicit collapse preference remains respected')
    assert.equal(tree.props['data-user-panel-collapsed'], preference === 'true' ? 'true' : undefined)
    if (preference === 'true') {
      descendants(tree).find(element => element.props['aria-label'] === '展开右侧用户栏').props.onClick()
      const restored = hooks.render(() => AiChatPage({ mode: 'work', onModeChange: noop }))
      assert.match(renderToStaticMarkup(restored), /全站用户/, 'the saved collapsed panel can be restored')
    }
  }
}

async function main() {
  testRoutesAndTransport()
  testSafeRecovery()
  await testLogin()
  testUserPanelPreference()
  console.log('PASS consumer entry: local/cloud routing, login/recovery, transport isolation, user panel preference')
}
main().catch(error => { console.error(error); process.exitCode = 1 })

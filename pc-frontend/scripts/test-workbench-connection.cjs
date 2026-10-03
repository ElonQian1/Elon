const assert = require('node:assert/strict')
const fs = require('node:fs')
const path = require('node:path')
const vm = require('node:vm')
const test = require('node:test')
const ts = require('typescript')

const root = path.resolve(__dirname, '..')
const ready = { status: 'ok', service: 'elon-server' }
const flush = async () => { for (let i = 0; i < 12; i++) await Promise.resolve() }
function load(relative, replacements = {}, globals = {}) {
  const filename = path.join(root, 'src', relative)
  const code = ts.transpileModule(fs.readFileSync(filename, 'utf8'), {
    fileName: filename,
    compilerOptions: { module: ts.ModuleKind.CommonJS, target: ts.ScriptTarget.ES2022,
      jsx: ts.JsxEmit.ReactJSX, esModuleInterop: true },
  }).outputText
  const exports = {}
  vm.runInNewContext(code, { exports, AbortController, Event, console,
    require: id => {
      if (Object.hasOwn(replacements, id)) return replacements[id]
      throw Error(`Unexpected dependency: ${id} from ${relative}`)
    }, ...globals }, { filename })
  return exports
}
function effects() {
  let cursor = 0
  const slots = [], pending = []
  const react = {
    useEffect(effect, deps) {
      const index = cursor++, old = slots[index]
      if (!old || deps.some((value, i) => !Object.is(value, old.deps[i]))) {
        pending.push(() => { old?.cleanup?.(); slots[index] = { deps, cleanup: effect() } })
      }
    },
    useState: value => [value, () => {}], lazy: () => 'Lazy', Suspense: 'Suspense',
  }
  return { react, render(fn) { cursor = 0; const tree = fn(); pending.splice(0).forEach(run => run()); return tree },
    unmount() { slots.forEach(slot => slot.cleanup?.()); slots.length = 0 } }
}
function createStore(initialize) {
  if (!initialize) return createStore
  let state
  const store = selector => selector ? selector(state) : state
  store.getState = () => state
  store.setState = update => { state = { ...state, ...(typeof update === 'function' ? update(state) : update) } }
  state = initialize(store.setState, store.getState)
  return store
}
function response(body = ready, status = 200, contentType = 'application/json') {
  return { ok: status >= 200 && status < 300, status,
    headers: { get: name => name === 'content-type' ? contentType : null },
    json: async () => { if (body instanceof Error) throw body; return body } }
}
function connectionHarness({ ignoreAbort = false } = {}) {
  const lifecycle = effects(), timers = new Map(), listeners = new Map(), requests = []
  let now = 0, nextTimer = 0, active = 0, peak = 0
  const browser = {
    setTimeout(fn, delay) { const id = ++nextTimer; timers.set(id, { at: now + delay, fn }); return id },
    clearTimeout(id) { timers.delete(id) },
    addEventListener(name, handler) { if (!listeners.has(name)) listeners.set(name, new Set()); listeners.get(name).add(handler) },
    removeEventListener(name, handler) { listeners.get(name)?.delete(handler) },
    dispatchEvent(event) { [...listeners.get(event.type) || []].forEach(handler => handler(event)); return true },
    location: Object.freeze({ pathname: '/pc/ai', assign() { throw Error('unexpected navigation') } }),
  }
  const fetch = (url, options) => {
    let settle, fail
    const promise = new Promise((resolve, reject) => { settle = resolve; fail = reject })
    const request = { url, options, resolve: settle, reject: fail }
    requests.push(request); active++; peak = Math.max(peak, active)
    options.signal.addEventListener('abort', () => { if (!ignoreAbort) fail(Error('aborted')) }, { once: true })
    return promise.finally(() => { active-- })
  }
  const model = load('features/shell/workbenchConnectionModel.ts', {}, { fetch })
  const hook = load('features/shell/useWorkbenchConnection.ts', {
    react: lifecycle.react, zustand: { create: createStore }, './workbenchConnectionModel': model,
    '../../api/runtime': { cloudConnectionProbeUrls: () => ['/ready', '/health'] },
  }, { window: browser })
  return { ...lifecycle, browser, requests, timers, model, hook,
    mount: (enabled = true) => lifecycle.render(() => hook.useWorkbenchConnectionMonitor(enabled)),
    state: () => hook.useWorkbenchConnection.getState(), peak: () => peak,
    async reply(index, body = ready, status = 200) { requests[index].resolve(response(body, status)); await flush() },
    async fail(index) { requests[index].reject(Error('network unavailable')); await flush() },
    async event(name) { browser.dispatchEvent(new Event(name)); await flush() },
    async advance(ms) {
      const end = now + ms
      while (true) {
        const item = [...timers].filter(([, value]) => value.at <= end).sort((a, b) => a[1].at - b[1].at)[0]
        if (!item) break
        const [id, value] = item; timers.delete(id); now = value.at; value.fn(); await flush()
      }
      now = end
    },
  }
}

test('readiness requires the real server JSON contract, not liveness, HTML, or JSON-shaped failures', async () => {
  for (const [body, status, contentType, expected] of [
    [ready, 200, 'application/json; charset=utf-8', true],
    [ready, 503, 'application/json', false],
    [ready, 200, 'text/html', false],
    [{ status: 'ok' }, 200, 'application/json', false],
    [{ status: 'starting', service: 'elon-server' }, 200, 'application/json', false],
    [{ status: 'ok', service: 'other-service' }, 200, 'application/json', false],
    [null, 200, 'application/json', false],
    [Error('invalid JSON'), 200, 'application/json', false],
  ]) {
    let observed
    const { probeWorkbenchCloud } = load('features/shell/workbenchConnectionModel.ts', {}, {
      fetch: async (url, options) => { observed = { url, options }; return response(body, status, contentType) },
    })
    const signal = new AbortController().signal
    assert.equal(await probeWorkbenchCloud('/ready', signal), expected, JSON.stringify({ body, status, contentType }))
    assert.equal(observed.options.signal, signal)
    assert.equal(observed.options.credentials, 'omit')
    assert.equal(observed.options.cache, 'no-store')
  }
})

test('first readiness failure is offline without falling back to a live /health endpoint', async () => {
  const h = connectionHarness(); h.mount()
  assert.equal(h.state().cloudState, 'checking')
  assert.equal(h.requests[0].url, '/ready')
  await h.fail(0)
  assert.equal(h.state().cloudState, 'offline')
  assert.equal(h.state().failedProbes, 1)
  assert.equal(h.state().checking, false)
  assert.equal(h.requests.length, 1)
  h.unmount()
})

test('an established connection tolerates three failures, goes offline on four, and resets after retry', async () => {
  const h = connectionHarness(); h.mount(); await h.reply(0)
  assert.equal(h.state().recoveryEpoch, 0)
  for (let failure = 1; failure <= 4; failure++) {
    await h.advance(h.model.CLOUD_PROBE_INTERVAL_MS); await h.fail(failure)
    assert.equal(h.state().cloudState, failure < 4 ? 'online' : 'offline')
    assert.equal(h.state().failedProbes, failure)
  }
  h.state().retry(); await h.reply(5)
  assert.equal(h.state().cloudState, 'online'); assert.equal(h.state().failedProbes, 0)
  assert.equal(h.state().recoveryEpoch, 1)
  await h.advance(h.model.CLOUD_PROBE_INTERVAL_MS); await h.reply(6)
  assert.equal(h.state().recoveryEpoch, 1, 'healthy polling must not repeatedly refresh recovered data')
  assert.equal(h.browser.location.pathname, '/pc/ai')
  h.unmount()
})

test('browser online and manual retry restore the same mounted workbench without overlapping probes', async () => {
  const h = connectionHarness(); h.mount(); await h.fail(0)
  await h.event('online'); assert.equal(h.requests.length, 2)
  h.state().retry(); await h.event('online')
  assert.equal(h.requests.length, 2)
  await h.reply(1); assert.equal(h.state().cloudState, 'online')
  assert.equal(h.peak(), 1); assert.equal(h.browser.location.pathname, '/pc/ai')
  h.unmount()
})

test('offline invalidates a pending success, and a later probe can recover in place', async () => {
  const h = connectionHarness({ ignoreAbort: true }); h.mount()
  await h.event('offline'); await h.reply(0)
  assert.equal(h.state().cloudState, 'offline')
  await h.event('online'); await h.reply(1)
  assert.equal(h.state().cloudState, 'online'); assert.equal(h.peak(), 1)
  h.unmount()
})

test('retry while in flight is coalesced into one immediate follow-up and offline/online aborts stale work', async () => {
  const h = connectionHarness(); h.mount()
  h.state().retry(); h.state().retry(); h.browser.dispatchEvent(new Event('online'))
  assert.equal(h.requests.length, 1)
  await h.reply(0)
  assert.equal(h.requests.length, 2, 'queued recovery must not wait for the polling interval')
  await h.reply(1); assert.equal(h.requests.length, 2); assert.equal(h.peak(), 1)
  await h.advance(h.model.CLOUD_PROBE_INTERVAL_MS)
  h.browser.dispatchEvent(new Event('offline')); h.browser.dispatchEvent(new Event('online'))
  await flush()
  assert.equal(h.requests[2].options.signal.aborted, true)
  assert.equal(h.requests.length, 4)
  await h.reply(3); assert.equal(h.state().cloudState, 'online'); assert.equal(h.peak(), 1)
  h.unmount()
})

test('timeout aborts the request and polling remains bounded', async () => {
  const h = connectionHarness(); h.mount()
  await h.advance(h.model.CLOUD_PROBE_TIMEOUT_MS)
  assert.equal(h.requests[0].options.signal.aborted, true)
  assert.equal(h.state().cloudState, 'offline')
  await h.advance(h.model.CLOUD_PROBE_INTERVAL_MS - 1); assert.equal(h.requests.length, 1)
  await h.advance(1); assert.equal(h.requests.length, 2); assert.equal(h.peak(), 1)
  h.unmount(); await flush()
})

test('disabled monitor has no I/O; cleanup aborts and ignores late results and future retry events', async () => {
  const h = connectionHarness({ ignoreAbort: true }); h.mount(false)
  assert.equal(h.requests.length, 0)
  h.mount(true); const before = h.state(); h.unmount()
  assert.equal(h.requests[0].options.signal.aborted, true)
  await h.reply(0); assert.equal(h.state(), before)
  await h.event('online'); h.state().retry(); await h.advance(60_000)
  assert.equal(h.requests.length, 1); assert.equal(h.timers.size, 0)
})

const jsx = { jsx: (type, props) => ({ type, props }), jsxs: (type, props) => ({ type, props }), Fragment: 'Fragment' }
function shellHarness({ locallyHosted = false, cloudState = 'online', duplicateTab = false } = {}) {
  const lifecycle = effects(), notifications = [], prewarm = [], authRequests = []
  const auth = createStore(() => ({ token: 'token-A', user: { id: 'A' },
    fetchMe: () => new Promise((resolve, reject) => authRequests.push({ resolve, reject })),
    logout() { auth.setState({ token: null, user: null }); logouts++ },
  }))
  let logouts = 0
  const connection = { cloudState }
  const noop = () => {}
  const dependencies = {
    react: lifecycle.react, 'react/jsx-runtime': jsx,
    'react-router-dom': { Link: 'Link', Outlet: 'Outlet' },
    'lucide-react': { CircleCheck: 'CircleCheck', Link2: 'Link2', TriangleAlert: 'TriangleAlert', WifiOff: 'WifiOff' },
    './desktopShell': { getDesktopInvoke: () => null, isDesktopShellFrameless: () => false },
    '../notifications/useNotifications': { useNotifications: enabled => notifications.push(enabled) },
    './useNodeAutoConnect': { useNodeAutoConnect: () => ({ status: 'success' }) },
    './useWorkbenchTabCoordinator': { useWorkbenchTabCoordinator: () => duplicateTab },
    '../../store/auth': { useAuthStore: auth },
    '../conversation/useProjectOpenPrewarm': { useProjectOpenPrewarm: enabled => prewarm.push(enabled) },
    '../../api/runtime': { isLocallyHostedWorkbench: () => locallyHosted },
    './useWorkbenchConnection': { useWorkbenchConnection: () => connection, useWorkbenchConnectionMonitor: noop },
    '../codex-control/useCodexControlBridge': { useCodexControlBridge: noop },
    '../browser-research/useBrowserResearchBridge': { useBrowserResearchBridge: noop },
  }
  for (const id of ['./ServerRail', './DesktopTitleBar', '../reader/ReaderTabStrip', '../reader/ReaderWorkspace',
    '../updates/AppUpdateWatcher', './LocalModeBanner', './Shell.module.css']) dependencies[id] = { __esModule: true, default: id }
  const { default: Shell } = load('features/shell/Shell.tsx', dependencies)
  return { ...lifecycle, auth, authRequests, notifications, prewarm, connection, logouts: () => logouts,
    renderShell: () => lifecycle.render(Shell) }
}

test('local and cloud hosting both refresh account and notifications; outage pauses them and recovery resumes', async () => {
  for (const locallyHosted of [false, true]) {
    const h = shellHarness({ locallyHosted }); h.renderShell()
    assert.equal(h.notifications.at(-1), true); assert.equal(h.authRequests.length, 1)
    h.authRequests[0].resolve(); await flush()
    h.connection.cloudState = 'offline'; h.renderShell()
    assert.equal(h.notifications.at(-1), false); assert.equal(h.authRequests.length, 1)
    h.connection.cloudState = 'online'; h.renderShell()
    assert.equal(h.notifications.at(-1), true); assert.equal(h.authRequests.length, 2)
    assert.equal(h.prewarm.at(-1), !locallyHosted)
    h.authRequests[1].resolve(); await flush(); h.unmount()
  }
  const duplicate = shellHarness({ duplicateTab: true }); duplicate.renderShell()
  assert.equal(duplicate.notifications.at(-1), false); assert.equal(duplicate.authRequests.length, 0)
  duplicate.unmount()
})

test('initial checking keeps cached identity and defers account refresh until readiness succeeds', async () => {
  for (const locallyHosted of [false, true]) {
    const h = shellHarness({ locallyHosted, cloudState: 'checking' }); h.renderShell()
    assert.equal(h.notifications.at(-1), true); assert.equal(h.authRequests.length, 0)
    assert.equal(h.auth.getState().user.id, 'A')
    h.connection.cloudState = 'online'; h.renderShell()
    assert.equal(h.authRequests.length, 1)
    h.renderShell(); assert.equal(h.authRequests.length, 1, 'healthy renders must not duplicate trusted-device refresh')
    h.authRequests[0].resolve(); await flush(); h.unmount()
  }
})

test('only a current account 401 logs out: network/403/500 and stale or cancelled failures preserve identity', async () => {
  for (const status of [undefined, 0, 403, 500, 401]) {
    const h = shellHarness({ locallyHosted: true }); h.renderShell()
    h.authRequests[0].reject({ status }); await flush()
    assert.equal(h.logouts(), status === 401 ? 1 : 0)
    assert.equal(h.auth.getState().token, status === 401 ? null : 'token-A')
    h.unmount()
  }
  const stale = shellHarness(); stale.renderShell()
  stale.auth.setState({ token: 'token-B', user: { id: 'B' } })
  stale.authRequests[0].reject({ status: 401 }); await flush()
  assert.equal(stale.logouts(), 0); assert.equal(stale.auth.getState().token, 'token-B')
  stale.unmount()
  const cancelled = shellHarness(); cancelled.renderShell(); cancelled.unmount()
  cancelled.authRequests[0].reject({ status: 401 }); await flush(); assert.equal(cancelled.logouts(), 0)
})

test('connection banner only presents outage, retries the shared monitor, and clears after recovery', () => {
  let retries = 0
  const state = { cloudState: 'online', checking: false, retry: () => { retries++ } }
  const { default: Banner } = load('features/shell/LocalModeBanner.tsx', {
    'react/jsx-runtime': jsx, 'lucide-react': { WifiOff: 'WifiOff' }, 'react-router-dom': { Link: 'Link' },
    './useWorkbenchConnection': { useWorkbenchConnection: () => state },
    './Shell.module.css': { __esModule: true, default: {} },
  })
  assert.equal(Banner(), null)
  state.cloudState = 'offline'
  const children = Banner().props.children
  const retry = children.find(child => child.type === 'button')
  assert.equal(retry.props.disabled, false); retry.props.onClick(); assert.equal(retries, 1)
  assert.deepEqual(Array.from(children.filter(child => child.type === 'Link'), child => child.props.to), ['/node'])
  state.checking = true; assert.equal(Banner().props.children.find(child => child.type === 'button').props.disabled, true)
  state.cloudState = 'online'; assert.equal(Banner(), null)
})

function authHarness() {
  const requests = []
  const request = (method, url) => new Promise((resolve, reject) => requests.push({ method, url, resolve, reject }))
  const { useAuthStore: store } = load('store/auth.ts', {
    zustand: { create: createStore }, 'zustand/middleware': { persist: initialize => initialize },
    '../api/client': { api: { get: url => request('GET', url), post: url => request('POST', url) } },
    '../features/node/localNodeApi': { nodeApi: async () => ({}) }, '../lib/utils': { safeNodeAdminUrl: () => '' },
    '../features/friends/socialChatCache': { clearSocialCaches() {} },
    '../features/friends/socialLocalState': { clearSocialLocalState() {} },
  }, { localStorage: { getItem: () => null, setItem() {}, removeItem() {} } })
  store.getState().acceptSession('token-A', 'expiry-A', { id: 'A' })
  return { store, requests, switchAccount() { store.getState().acceptSession('token-B', 'expiry-B', { id: 'B' }) } }
}

test('actual auth refresh rejects stale success before user mutation or trusted-device request', async () => {
  const h = authHarness(), refresh = h.store.getState().fetchMe()
  assert.equal(h.requests[0].url, '/api/me')
  h.switchAccount(); h.requests[0].resolve({ user: { id: 'A-old' } }); await refresh
  assert.equal(h.store.getState().token, 'token-B'); assert.equal(h.store.getState().user.id, 'B')
  assert.equal(h.store.getState().expiresAt, 'expiry-B')
  assert.equal(h.requests.length, 1, 'an obsolete identity must not renew the new account device trust')
})

test('actual auth refresh ignores old trust expiry and preserves session on network rejection', async () => {
  const h = authHarness(), refresh = h.store.getState().fetchMe()
  h.requests[0].resolve({ user: { id: 'A-current' } }); await flush()
  assert.equal(h.requests[1].url, '/api/auth/trust-current-device')
  h.switchAccount(); h.requests[1].resolve({ expires_at: 'old-account-expiry' }); await refresh
  assert.equal(h.store.getState().expiresAt, 'expiry-B'); assert.equal(h.store.getState().user.id, 'B')
  const failed = h.store.getState().fetchMe(), networkError = Error('network unavailable')
  h.requests[2].reject(networkError); await assert.rejects(failed, error => error === networkError)
  assert.equal(h.store.getState().token, 'token-B'); assert.equal(h.store.getState().expiresAt, 'expiry-B')
})

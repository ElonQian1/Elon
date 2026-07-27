const assert = require('node:assert');
const fs = require('node:fs');
const path = require('node:path');
const vm = require('node:vm');

const repoRoot = path.resolve(__dirname, '..');
const mobileWeb = fs.readFileSync(path.join(repoRoot, 'server/src/assets/web_page.html'), 'utf8');
const authBootstrap = fs.readFileSync(path.join(repoRoot, 'server/src/assets/ui_tuner_pwa_auth_bootstrap.js'), 'utf8');
const sourceVerification = fs.readFileSync(path.join(repoRoot, 'server/src/assets/ui_tuner_pwa_verification.js'), 'utf8');
const bridge = fs.readFileSync(path.join(repoRoot, 'server/src/assets/ui_tuner_pwa_bridge.js'), 'utf8');
const previewSurface = fs.readFileSync(path.join(repoRoot, 'pc-frontend/src/features/ui-tuner/source-preview/PwaInteractivePreviewSurface.tsx'), 'utf8');
const previewSurfaceCss = fs.readFileSync(path.join(repoRoot, 'pc-frontend/src/features/ui-tuner/source-preview/SourcePreview.module.css'), 'utf8');
const draftContract = fs.readFileSync(path.join(repoRoot, 'pc-frontend/src/features/ui-tuner/source-preview/pwaDesignDraft.ts'), 'utf8');
const designSessionModel = fs.readFileSync(path.join(repoRoot, 'pc-frontend/src/features/ui-tuner/source-preview/pwaDesignSessionModel.ts'), 'utf8');
const designSession = fs.readFileSync(path.join(repoRoot, 'pc-frontend/src/features/ui-tuner/source-preview/usePwaDesignSession.ts'), 'utf8');
const styleInspector = fs.readFileSync(path.join(repoRoot, 'pc-frontend/src/features/ui-tuner/source-preview/PwaStyleInspector.tsx'), 'utf8');

assert.ok(mobileWeb.includes('__UI_TUNER_PWA_BRIDGE_JS__'), 'mobile page should embed the isolated PWA design bridge');
assert.ok(mobileWeb.includes('__UI_TUNER_PWA_VERIFICATION_JS__'), 'mobile page should embed isolated real-source verification');
assert.ok(mobileWeb.indexOf('__UI_TUNER_PWA_AUTH_BOOTSTRAP_JS__') < mobileWeb.indexOf("const TOKEN_KEY = 'lodex_token'"), 'preview auth must be inherited before the mobile app boots');
assert.ok(authBootstrap.includes("localStorage.getItem('elon_auth')"), 'same-origin preview should inherit the current PC session synchronously');
assert.ok(mobileWeb.includes('if (token !== bootToken) return;'), 'a stale boot must not clear a newer bridged session');
assert.match(mobileWeb, /currentUser = data\.user;\s+syncWebSideMenuUser\(\);/, 'successful preview auth should immediately refresh the visible project sidebar account');
assert.ok(bridge.includes("params.get('ui_tuner_preview') !== '1'"), 'normal PWA pages must not activate the design bridge');
assert.ok(bridge.includes("const SOURCE = 'elon-pwa-design-bridge'"), 'PWA should expose its design bridge to the PC workbench');
assert.ok(bridge.includes("event.origin !== window.location.origin"), 'PWA design bridge must reject cross-origin commands');
assert.ok(bridge.includes("message.type === 'set-mode'"), 'PWA should switch between component selection and normal interaction');
assert.ok(bridge.includes("message.type === 'apply-style'"), 'PWA should apply immediate local style previews');
assert.ok(bridge.includes("message.type === 'reset-styles'"), 'PWA should reset ephemeral preview styles');
assert.ok(bridge.includes("message.type === 'verify-source'"), 'PWA should verify only after clearing ephemeral preview styles');
assert.ok(sourceVerification.includes("type: 'source-verification'"), 'real source verification should return an explicit bridge snapshot');
assert.ok(bridge.includes("message.type === 'reset-element'"), 'PWA should reset the selected real DOM element');
assert.ok(bridge.includes("message.type === 'apply-draft'"), 'PWA should restore structured page drafts');
assert.ok(bridge.includes("strategy: 'dom-path'"), 'unbound DOM elements should expose an explainable path identity');
assert.ok(bridge.includes('needsBinding: true'), 'generated identities must not pretend to be source bindings');
assert.ok(bridge.includes("getAttribute('data-ui-style-binding')"), 'only an explicit DOM source binding may enable deterministic PWA writeback');
assert.ok(bridge.includes("'lineHeight'"), 'PWA should support the first manual typography property set');
assert.ok(bridge.includes('let selecting = false'), 'PWA design bridge must default to real interaction');
assert.ok(bridge.includes("document.addEventListener('click', handleDesignClick, true)"), 'design mode should install its capture listener explicitly');
assert.ok(bridge.includes("document.removeEventListener('click', handleDesignClick, true)"), 'interaction mode should remove the design capture listener');
assert.ok(bridge.includes("mode: 'interact'"), 'PWA ready event must report interaction mode');
assert.ok(bridge.includes("message.type === 'set-session-auth'"), 'preview should accept an ephemeral same-origin session bridge');
assert.ok(bridge.includes("CustomEvent('elon:ui-tuner-session-auth'"), 'session auth must stay in page memory');
assert.ok(!bridge.includes("localStorage.setItem('lodex_token'"), 'preview bridge must not persist the PC token');
assert.ok(bridge.includes("post('route-changed'"), 'PWA should report route changes without parent reloads');
assert.ok(bridge.includes("getAttribute('data-ui-screen')"), 'PWA should prefer an explicit screen identity when the app supplies one');
assert.ok(bridge.includes("document.querySelectorAll('.page.active[id]')"), 'PWA should fall back to the active page id without depending on DOM order');
assert.ok(bridge.includes("document.querySelector('#topTitle')"), 'PWA should distinguish active screens by their stable visible title');
assert.ok(bridge.includes('new MutationObserver'), 'PWA should watch semantic screen changes without reloading');
assert.match(bridge, /attributeFilter:\s*\[[^\]]*'class'[^\]]*'aria-hidden'[^\]]*'data-ui-screen'[^\]]*\]/, 'screen observer should watch semantic state attributes');
assert.doesNotMatch(bridge.match(/attributeFilter:\s*\[[^\]]*\]/)?.[0] || '', /['"]style['"]/, 'screen observer must never subscribe to inline style changes');
assert.ok(mobileWeb.includes("pageParams.get('ui_tuner_preview') === '1'"), 'session auth listener must be preview-scoped');
assert.ok(designSession.includes("useState<'select' | 'interact'>('interact')"), 'PC PWA canvas must default to real interaction');
assert.ok(designSession.includes("context.post('set-session-auth', { token })"), 'PC should bridge the current same-origin login session');
assert.ok(designSession.includes("message.type === 'draft-applied'"), 'PC should wait for the real iframe draft acknowledgement');
assert.ok(designSession.includes('identity: { ...element.identity'), 'PC should send a stable element identity with every draft entry');
assert.ok(designSession.includes('resolvePwaStyleBinding'), 'PC should resolve matching runtime CSS selectors through the local source node');
assert.equal((designSession.match(/window\.addEventListener\('message'/g) || []).length, 1, 'PC session should declare one postMessage listener');
assert.match(designSession, /const bridgeContextRef = useRef\([\s\S]*window\.addEventListener\('message', receive\)[\s\S]*window\.removeEventListener\('message', receive\)\s*\n\s*}, \[\]\)/, 'PC session listener should stay installed while refs provide current render state');
assert.ok(previewSurface.includes('key={design.reloadKey}'), 'iframe reloads must remain explicit and controlled by source verification');
assert.ok(previewSurface.includes("url.searchParams.set('ui_tuner_reload'"), 'source verification reload should use an explicit cache buster');
assert.ok(previewSurface.includes('开始设计/修改页面'), 'manual design mode should have one clear Chinese entry point');
const layoutOrder = ['pwaWorkflowGuide', 'pwaPreviewToolbar', 'pwaRouteStatus', 'pwaDraftBadge', 'pwaDeviceViewport', 'pwaDeviceFrame']
  .map((className) => previewSurface.indexOf(`styles.${className}`));
assert.ok(layoutOrder.every((index) => index >= 0), 'PWA preview chrome and iframe must all remain rendered');
assert.deepEqual([...layoutOrder].sort((left, right) => left - right), layoutOrder, 'workflow, toolbar, route, and badge must stay before the iframe viewport in normal flow');
assert.match(previewSurface, /className=\{styles\.pwaDraftBadge\}>[^<]+<\/div>\s*<div className=\{styles\.pwaDeviceViewport\}/, 'the draft badge must be a sibling before the iframe viewport');
assert.match(previewSurface, /className=\{styles\.pwaDeviceViewport\}[^>]*>\s*<iframe/, 'the iframe viewport must not contain overlay chrome above the real page');
for (const className of ['pwaWorkflowGuide', 'pwaPreviewToolbar', 'pwaRouteStatus', 'pwaDraftBadge']) {
  const rule = previewSurfaceCss.match(new RegExp(`\\.${className}\\s*\\{[^}]*\\}`))?.[0] ?? '';
  assert.ok(rule, `${className} should keep an explicit layout rule`);
  assert.doesNotMatch(rule, /position\s*:\s*(?:absolute|fixed|sticky)/, `${className} must not overlay the iframe`);
  assert.doesNotMatch(rule, /(?:top|margin-top)\s*:\s*-/, `${className} must not use a negative top offset toward the iframe`);
}
assert.ok(!bridge.includes("document.addEventListener('pointerover'"), 'PWA selection must not recalculate layout on every mouse hover');
assert.ok(!bridge.includes("String(element.innerText || element.textContent || '')"), 'PWA selection must not read a large container innerText on every click');
assert.ok(draftContract.includes("kind: 'elon.pwa.manual_style_draft'"), 'PC should persist a typed manual PWA draft contract');
assert.ok(draftContract.includes('originalStyle: PwaOriginalStyleSnapshot'), 'draft elements should retain their original style snapshot');
assert.ok(draftContract.includes('confidenceScore: number'), 'draft identities should retain mapping confidence');
assert.ok(draftContract.includes("params.delete('ui_tuner_preview')"), 'draft route identity should ignore the preview-only query flag');
assert.ok(draftContract.includes('screenKey?: string'), 'draft schema v2 should accept an optional real screen identity');
assert.ok(draftContract.includes("route.screenKey || 'screen:unidentified'"), 'draft storage keys should include the real screen identity');
assert.ok(designSessionModel.includes('readPwaDesignDraft(project, route)'), 'PC reload should restore the matching project, route, and viewport draft');
assert.ok(designSessionModel.includes('PWA_DESIGN_TRANSACTION_IDLE_MS = 450'), 'continuous controls should be grouped into editing transactions');
assert.ok(designSessionModel.includes('pastDrafts') && designSessionModel.includes('futureDrafts'), 'manual draft editing should support undo and redo');
assert.ok(designSession.includes("model.update('page:clear'"), 'clearing the current page should remain undoable');
assert.ok(designSession.includes("draft.route.screenKey || 'screen:unidentified'"), 'bridge draft keys should include screenKey');
assert.ok(designSession.includes("normalized.screenKey || 'screen:unidentified'"), 'PWA session route keys should include screenKey');
assert.ok(previewSurface.includes('当前画面：'), 'PC should show the user-readable current screen title');
for (const property of ['width', 'height', 'paddingTop', 'marginTop', 'borderRadius', 'fontSize', 'fontWeight', 'lineHeight', 'color', 'backgroundColor', 'opacity']) {
  assert.ok(styleInspector.includes(`'${property}'`), `manual style panel should expose ${property}`);
}

function runAuthBootstrap() {
  const parent = {};
  const window = {
    location: { search: '?ui_tuner_preview=1' },
    parent,
    localStorage: {
      getItem: () => JSON.stringify({ state: { token: 'pc-session-token' } }),
    },
  };
  vm.runInNewContext(authBootstrap, { window, URLSearchParams, Object });
  assert.equal(window.__ELON_UI_TUNER_PREVIEW_AUTH__.token, 'pc-session-token');
}

class FakeClassList {
  constructor() { this.values = new Set(); }
  add(...values) { values.forEach((value) => this.values.add(value)); }
  remove(...values) { values.forEach((value) => this.values.delete(value)); }
  filter(callback) { return Array.from(this.values).filter(callback); }
  contains(value) { return this.values.has(value); }
  [Symbol.iterator]() { return this.values[Symbol.iterator](); }
  toggle(value, enabled) {
    if (enabled) this.values.add(value);
    else this.values.delete(value);
  }
}

class FakeStyle {
  constructor() { this.values = new Map(); this.setCount = 0; }
  getPropertyValue(property) { return this.values.get(property) || ''; }
  setProperty(property, value) { this.setCount += 1; this.values.set(property, String(value)); }
  removeProperty(property) { this.values.delete(property); }
}

class FakeElement {
  constructor(tagName, id = '') {
    this.tagName = tagName.toUpperCase();
    this.id = id;
    this.attributes = new Map();
    this.classList = new FakeClassList();
    this.style = new FakeStyle();
    this.children = [];
    this.childNodes = [];
    this.parentElement = null;
    this.isConnected = true;
    this.listeners = new Map();
    this.matchesCount = 0;
  }
  appendChild(child) { child.parentElement = this; this.children.push(child); return child; }
  remove() {
    if (this.parentElement) this.parentElement.children = this.parentElement.children.filter((child) => child !== this);
    this.parentElement = null;
    this.isConnected = false;
  }
  addEventListener(type, listener) {
    const listeners = this.listeners.get(type) || [];
    listeners.push(listener);
    this.listeners.set(type, listeners);
  }
  getAttribute(name) {
    if (name === 'id') return this.id || null;
    if (name === 'style') return this.attributes.get(name) ?? null;
    return this.attributes.get(name) ?? null;
  }
  setAttribute(name, value) {
    if (name === 'id') this.id = String(value);
    this.attributes.set(name, String(value));
  }
  removeAttribute(name) {
    this.attributes.delete(name);
    if (name === 'style') this.style = new FakeStyle();
  }
  closest(selector) {
    if (selector === '[data-component]') return null;
    if (selector.includes('[id]') && this.id) return this;
    return null;
  }
  matches(selector) {
    this.matchesCount += 1;
    return selector === '.page-title' && this.id === 'topTitle';
  }
  getBoundingClientRect() { return { left: 12, top: 24, width: 180, height: 48 }; }
  querySelectorAll(selector) {
    if (selector !== '[data-ui-screen]') return [];
    return this.children.filter((child) => child.getAttribute('data-ui-screen'));
  }
}

function runBridgeBehavior() {
  const parent = { postMessage: (message) => posted.push(message) };
  const posted = [];
  const windowListeners = new Map();
  const documentListeners = new Map();
  const authEvents = [];
  const pendingTimers = new Map();
  let nextTimer = 1;
  let screenObserver = null;
  const documentElement = new FakeElement('html');
  const body = new FakeElement('body');
  const title = new FakeElement('h1', 'topTitle');
  title.childNodes.push({ nodeType: 3, textContent: '好友' });
  title.setAttribute('data-ui-style-binding', JSON.stringify({
    version: 1,
    sourceFile: 'src/styles/title.css',
    sourceRevision: 'a'.repeat(64),
    kind: 'css-rule',
    target: '.page-title',
    range: { start: 120, end: 180 },
    propertyMap: { fontSize: 'font-size', color: 'color' },
  }));
  body.appendChild(title);
  const chatPage = new FakeElement('div', 'chatPage');
  chatPage.classList.add('page', 'active');
  const projectPage = new FakeElement('div', 'projectPage');
  projectPage.classList.add('page');
  body.appendChild(chatPage);
  body.appendChild(projectPage);
  const setTitle = (value) => { title.childNodes = [{ nodeType: 3, textContent: value }]; };
  const findById = (root, id) => {
    if (root.id === id) return root;
    for (const child of root.children) {
      const found = findById(child, id);
      if (found) return found;
    }
    return null;
  };
  const document = {
    title: '一龙',
    documentElement,
    body,
    styleSheets: [],
    createElement: (tagName) => new FakeElement(tagName),
    addEventListener: (type, listener) => {
      const listeners = documentListeners.get(type) || [];
      listeners.push(listener);
      documentListeners.set(type, listeners);
    },
    removeEventListener: (type, listener) => {
      const listeners = documentListeners.get(type) || [];
      documentListeners.set(type, listeners.filter((candidate) => candidate !== listener));
    },
    querySelectorAll: (selector) => {
      if (selector === '#topTitle') return [title];
      if (selector === '.page.active[id]') return [chatPage, projectPage].filter((page) => page.classList.contains('active'));
      if (selector === '[data-ui-screen].active, [data-ui-screen][aria-hidden="false"]') {
        return [chatPage, projectPage].filter((page) => (
          page.getAttribute('data-ui-screen')
          && (page.classList.contains('active') || page.getAttribute('aria-hidden') === 'false')
        ));
      }
      return [];
    },
    querySelector: (selector) => {
      if (selector.startsWith('#')) return findById(body, selector.slice(1));
      return null;
    },
  };
  document.styleSheets.push({
    cssRules: [{
      selectorText: '.page-title',
      style: { getPropertyValue: (property) => property === 'font-size' ? '18px' : '' },
    }],
  });
  const computedStyle = {
    width: '180px', height: '48px', paddingTop: '0px', paddingRight: '0px',
    paddingBottom: '0px', paddingLeft: '0px', marginTop: '0px', marginRight: '0px',
    marginBottom: '0px', marginLeft: '0px', borderRadius: '0px', fontSize: '18px',
    fontWeight: '700', lineHeight: '24px', color: 'rgb(17, 24, 39)',
    backgroundColor: 'rgba(0, 0, 0, 0)', opacity: '1',
  };
  const window = {
    location: {
      search: '?ui_tuner_preview=1', origin: 'https://elon.example', href: 'https://elon.example/web?ui_tuner_preview=1',
      pathname: '/web', hash: '',
    },
    parent,
    innerWidth: 390,
    innerHeight: 844,
    scrollX: 0,
    scrollY: 0,
    getComputedStyle: () => computedStyle,
    addEventListener: (type, listener) => {
      const listeners = windowListeners.get(type) || [];
      listeners.push(listener);
      windowListeners.set(type, listeners);
    },
    dispatchEvent: (event) => authEvents.push(event),
    setTimeout: (callback) => {
      const timer = nextTimer++;
      pendingTimers.set(timer, callback);
      return timer;
    },
    clearTimeout: (timer) => pendingTimers.delete(timer),
  };
  const history = {
    pushState() {},
    replaceState() {},
  };
  class FakeCustomEvent {
    constructor(type, options) { this.type = type; this.detail = options.detail; }
  }
  class FakeMutationObserver {
    constructor(callback) { this.callback = callback; screenObserver = this; }
    observe(target, options) { this.target = target; this.options = options; }
    disconnect() {}
  }
  const flushTimers = () => {
    const callbacks = Array.from(pendingTimers.values());
    pendingTimers.clear();
    callbacks.forEach((callback) => callback());
  };
  const context = {
    window, document, history, URLSearchParams,
    Node: { TEXT_NODE: 3 }, Element: FakeElement,
    CSS: { escape: (value) => value }, CustomEvent: FakeCustomEvent,
    MutationObserver: FakeMutationObserver,
  };
  vm.runInNewContext(sourceVerification, context);
  vm.runInNewContext(bridge, context);

  assert.equal(windowListeners.get('message').length, 1, 'bridge should install exactly one command listener');
  assert.equal(documentListeners.get('click')?.length || 0, 0, 'normal interaction must not install a capture listener');
  assert.equal(body.children.some((child) => child.id === 'uiTunerPreviewSelection'), false, 'normal interaction must not create a selection layer');
  assert.equal(posted.filter((message) => message.type === 'ready').length, 1, 'bridge should emit one ready event');
  assert.equal(posted.filter((message) => message.type === 'route-changed').length, 1, 'bridge should emit one initial route event');
  assert.equal(posted.find((message) => message.type === 'route-changed').payload.screenKey, 'page:chatPage|title:好友');
  assert.equal(screenObserver.target, body, 'screen observer should watch the rendered PWA body');
  assert.ok(screenObserver.options.characterData, 'screen observer should watch visible title text');
  assert.ok(!screenObserver.options.attributeFilter.includes('style'), 'screen observer should exclude style mutations');

  const projectRow = new FakeElement('button', 'projectRow');
  body.appendChild(projectRow);
  let navigationCount = 0;
  projectRow.addEventListener('click', () => { navigationCount += 1; });
  const dispatchClick = (target) => {
    let stopped = false;
    const event = {
      target,
      defaultPrevented: false,
      preventDefault() { this.defaultPrevented = true; },
      stopImmediatePropagation() { stopped = true; },
    };
    for (const listener of documentListeners.get('click') || []) listener(event);
    if (!stopped) for (const listener of target.listeners.get('click') || []) listener(event);
    return event;
  };
  const normalClick = dispatchClick(projectRow);
  assert.equal(normalClick.defaultPrevented, false, 'normal interaction must not prevent a real project-row click');
  assert.equal(navigationCount, 1, 'one real project-row click must navigate exactly once');
  assert.equal(posted.filter((message) => message.type === 'selection').length, 0, 'normal interaction must not turn a project-row click into a design selection');

  const command = (type, payload) => windowListeners.get('message')[0]({
    origin: window.location.origin,
    source: parent,
    data: { source: 'elon-pc-ui-tuner', protocolVersion: 1, type, payload },
  });
  command('set-session-auth', { token: 'pc-session-token' });
  command('set-session-auth', { token: 'pc-session-token' });
  assert.equal(authEvents.length, 1, 'repeated auth commands must not restart the PWA boot loop');
  assert.equal(authEvents[0].detail.token, 'pc-session-token');

  command('set-mode', { mode: 'select' });
  assert.equal(documentListeners.get('click').length, 1, 'design mode should install exactly one capture listener');
  assert.equal(body.children.filter((child) => child.id === 'uiTunerPreviewSelection').length, 1, 'design mode should create exactly one selection layer');
  const designClick = dispatchClick(projectRow);
  assert.equal(designClick.defaultPrevented, true, 'design mode must prevent the real navigation click');
  assert.equal(navigationCount, 1, 'design mode should select without navigating');
  assert.equal(posted.filter((message) => message.type === 'selection').length, 1, 'the iframe top title must remain clickable and selectable in design mode');
  const selectedBinding = posted.filter((message) => message.type === 'selection').at(-1).payload.node.sourceBinding;
  assert.equal(selectedBinding, null, 'an unbound project row must not invent a source binding');

  command('set-mode', { mode: 'interact' });
  assert.equal(documentListeners.get('click')?.length || 0, 0, 'exiting design must immediately remove the capture listener');
  assert.equal(body.children.some((child) => child.id === 'uiTunerPreviewSelection'), false, 'exiting design must remove the selection layer');
  dispatchClick(projectRow);
  assert.equal(navigationCount, 2, 'the first click after exiting design must navigate exactly once');

  command('set-mode', { mode: 'select' });
  const matchesBeforeTitleSelection = title.matchesCount;
  dispatchClick(title);
  assert.equal(title.matchesCount - matchesBeforeTitleSelection, 1, 'one selection should scan each stylesheet rule once instead of once per editable property');
  const titleBinding = posted.filter((message) => message.type === 'selection').at(-1).payload.node.sourceBinding;
  assert.equal(titleBinding.sourceFile, 'src/styles/title.css', 'selection should carry the explicit safe source file');
  assert.deepEqual(titleBinding.propertyMap, { fontSize: 'font-size', color: 'color' });
  assert.deepEqual(
    posted.filter((message) => message.type === 'selection').at(-1).payload.node.sourceSelectors,
    ['.page-title'],
    'selection should expose the actual matching CSS rule for local deterministic resolution',
  );

  const messagesBeforeStyle = posted.length;
  command('apply-style', { selector: '#topTitle', style: { fontSize: '22px' } });
  assert.equal(title.style.getPropertyValue('font-size'), '22px', 'font size must update synchronously on the real DOM');
  assert.equal(posted.length, messagesBeforeStyle + 1, 'one style command should produce one acknowledgement without a message loop');
  assert.equal(posted.at(-1).type, 'style-applied');
  assert.equal(posted.filter((message) => message.type === 'route-changed').length, 1, 'editing inline style must not emit a screen change');

  const draft = {
    draftKey: 'project-1|/web|||390x844', revision: 7,
    elements: [{
      selector: '#topTitle',
      identity: { key: 'id:topTitle', id: 'topTitle', tag: 'h1' },
      styleDiff: { fontSize: '24px' },
    }],
  };
  const acknowledgementsBeforeDraft = posted.filter((message) => message.type === 'draft-applied').length;
  command('apply-draft', draft);
  assert.equal(title.style.getPropertyValue('font-size'), '24px', 'one draft property write must update the selected DOM element');
  assert.equal(posted.filter((message) => message.type === 'draft-applied').length, acknowledgementsBeforeDraft + 1, 'one draft revision should receive one acknowledgement');
  assert.deepEqual(posted.at(-1).payload, {
    requestedCount: 1, appliedCount: 1, unresolved: [], complete: true,
    draftKey: draft.draftKey, revision: 7, attempt: 1, maxAttempts: 8, retrying: false, exhausted: false,
  }, 'a complete acknowledgement should report real requested and applied counts');
  const completeDraftSetCount = title.style.setCount;
  command('apply-draft', draft);
  assert.equal(posted.filter((message) => message.type === 'draft-applied').length, acknowledgementsBeforeDraft + 2, 'a repeated draft revision should replay its acknowledgement');
  assert.equal(title.style.setCount, completeDraftSetCount, 'a repeated draft revision must not write inline styles twice');

  const lateTargetDraft = {
    draftKey: draft.draftKey, revision: 8,
    elements: [{
      selector: '#lateTarget',
      identity: { key: 'id:lateTarget', id: 'lateTarget', tag: 'section' },
      styleDiff: { borderRadius: '18px' },
    }],
  };
  command('apply-draft', lateTargetDraft);
  let draftAck = posted.filter((message) => message.type === 'draft-applied').at(-1).payload;
  assert.equal(draftAck.complete, false, 'an async target missing on first delivery must stay incomplete');
  assert.equal(draftAck.appliedCount, 0);
  assert.equal(draftAck.requestedCount, 1);
  assert.equal(draftAck.unresolved[0].reason, 'target-missing');
  const lateTarget = new FakeElement('section', 'lateTarget');
  body.appendChild(lateTarget);
  screenObserver.callback([{ type: 'childList' }]);
  flushTimers();
  draftAck = posted.filter((message) => message.type === 'draft-applied').at(-1).payload;
  assert.equal(lateTarget.style.getPropertyValue('border-radius'), '18px', 'a target added after the draft must receive its style');
  assert.equal(draftAck.complete, true, 'the same revision completes only after the async target is applied');
  assert.equal(draftAck.appliedCount, 1);
  assert.deepEqual(draftAck.unresolved, []);

  const neverAppearsDraft = {
    draftKey: draft.draftKey, revision: 9,
    elements: [{
      selector: '#neverAppears',
      identity: { key: 'id:neverAppears', id: 'neverAppears', tag: 'div' },
      styleDiff: { borderRadius: '18px' },
    }],
  };
  command('apply-draft', neverAppearsDraft);
  for (let attempt = 0; attempt < 12; attempt += 1) flushTimers();
  draftAck = posted.filter((message) => message.type === 'draft-applied').at(-1).payload;
  assert.equal(draftAck.complete, false, 'a target that never appears must not produce false success');
  assert.equal(draftAck.exhausted, true, 'missing-target retries must stop at the explicit bound');
  assert.equal(draftAck.attempt, 8);
  assert.equal(pendingTimers.size, 0, 'exhausted draft retries must leave no infinite timer loop');
  const acksAfterExhaustion = posted.filter((message) => message.type === 'draft-applied').length;
  screenObserver.callback([{ type: 'childList' }]);
  flushTimers();
  assert.equal(posted.filter((message) => message.type === 'draft-applied').length, acksAfterExhaustion, 'later mutations must not restart an exhausted revision');

  const driftedTarget = new FakeElement('button', 'driftedTarget');
  body.appendChild(driftedTarget);
  command('apply-draft', {
    draftKey: draft.draftKey, revision: 10,
    elements: [{
      selector: '#driftedTarget',
      identity: { key: 'id:originalTarget', id: 'originalTarget', tag: 'button' },
      styleDiff: { borderRadius: '18px' },
    }],
  });
  draftAck = posted.filter((message) => message.type === 'draft-applied').at(-1).payload;
  assert.equal(driftedTarget.style.getPropertyValue('border-radius'), '', 'selector drift must never modify the wrong element');
  assert.equal(draftAck.complete, false);
  assert.equal(draftAck.unresolved[0].reason, 'identity-mismatch');
  assert.equal(draftAck.exhausted, true, 'identity mismatch is a terminal safe rejection, not an infinite retry');

  const legacyUnsafeTarget = new FakeElement('div', 'legacyUnsafeTarget');
  body.appendChild(legacyUnsafeTarget);
  command('apply-draft', {
    draftKey: draft.draftKey, revision: 11,
    elements: [{ selector: '#legacyUnsafeTarget', identity: { tag: 'div' }, styleDiff: { borderRadius: '18px' } }],
  });
  draftAck = posted.filter((message) => message.type === 'draft-applied').at(-1).payload;
  assert.equal(legacyUnsafeTarget.style.getPropertyValue('border-radius'), '', 'a legacy selector without identity evidence must fail closed');
  assert.equal(draftAck.unresolved[0].reason, 'identity-insufficient');

  const verificationsBefore = posted.filter((message) => message.type === 'source-verification').length;
  command('verify-source', {
    requestId: 'verify-title-r7',
    checks: [{ elementKey: 'title', selector: '#topTitle', properties: ['fontSize'] }],
  });
  assert.equal(title.style.getPropertyValue('font-size'), '', 'real source verification must clear the temporary draft first');
  const sourceSnapshot = posted.filter((message) => message.type === 'source-verification').at(-1);
  assert.equal(posted.filter((message) => message.type === 'source-verification').length, verificationsBefore + 1, 'one verify command should return exactly one snapshot');
  assert.equal(sourceSnapshot.payload.nodes[0].computed.fontSize, '18px', 'snapshot must read computed style from the source page after reset');
  assert.deepEqual(sourceSnapshot.payload.changedFiles, [], 'snapshot must return changed-files evidence even when the page exposes no metadata');
  assert.deepEqual(sourceSnapshot.payload.sourceRevisions, {}, 'snapshot must return source revisions even when the page exposes no metadata');
  command('verify-source', {
    requestId: 'verify-title-r7',
    checks: [{ elementKey: 'title', selector: '#topTitle', properties: ['fontSize'] }],
  });
  assert.equal(posted.filter((message) => message.type === 'source-verification').length, verificationsBefore + 1, 'a repeated verification request must not produce a message loop');
  command('apply-draft', draft);
  assert.equal(title.style.getPropertyValue('font-size'), '24px', 'a failed verification can restore the same draft revision');
  assert.equal(posted.filter((message) => message.type === 'draft-applied').at(-1).payload.complete, true, 'restoring the same draft revision should complete after reset');

  const routeCountBeforeScreenSwitch = posted.filter((message) => message.type === 'route-changed').length;
  chatPage.classList.toggle('active', false);
  projectPage.classList.toggle('active', true);
  setTitle('一龙项目');
  screenObserver.callback([{ type: 'attributes', attributeName: 'class' }]);
  screenObserver.callback([{ type: 'characterData' }]);
  flushTimers();
  let routeMessages = posted.filter((message) => message.type === 'route-changed');
  assert.equal(routeMessages.length, routeCountBeforeScreenSwitch + 1, 'one real screen switch should be debounced into one route message');
  assert.equal(routeMessages.at(-1).payload.screenKey, 'page:projectPage|title:一龙项目');
  assert.equal(routeMessages.at(-1).payload.screenTitle, '一龙项目');

  screenObserver.callback([{ type: 'attributes', attributeName: 'class' }]);
  flushTimers();
  routeMessages = posted.filter((message) => message.type === 'route-changed');
  assert.equal(routeMessages.length, routeCountBeforeScreenSwitch + 1, 'an unchanged screen signature must not emit repeatedly');

  setTitle('演示项目');
  screenObserver.callback([{ type: 'characterData' }]);
  flushTimers();
  routeMessages = posted.filter((message) => message.type === 'route-changed');
  assert.equal(routeMessages.at(-1).payload.screenKey, 'page:projectPage|title:演示项目', 'different project titles on projectPage should produce isolated screen keys');

  projectPage.setAttribute('data-ui-screen', 'Project:Stable-42');
  projectPage.setAttribute('data-ui-screen-title', '稳定项目画面');
  screenObserver.callback([{ type: 'attributes', attributeName: 'data-ui-screen' }]);
  flushTimers();
  routeMessages = posted.filter((message) => message.type === 'route-changed');
  assert.equal(routeMessages.at(-1).payload.screenKey, 'data-ui-screen:project:stable-42', 'explicit data-ui-screen should override the page/title fallback');
  assert.equal(routeMessages.at(-1).payload.screenTitle, '稳定项目画面');
}

runAuthBootstrap();
runBridgeBehavior();

console.log('PWA UI tuner bridge tests passed');

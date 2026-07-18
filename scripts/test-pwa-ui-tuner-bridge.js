const assert = require('node:assert');
const fs = require('node:fs');
const path = require('node:path');
const vm = require('node:vm');

const repoRoot = path.resolve(__dirname, '..');
const mobileWeb = fs.readFileSync(path.join(repoRoot, 'server/src/assets/web_page.html'), 'utf8');
const authBootstrap = fs.readFileSync(path.join(repoRoot, 'server/src/assets/ui_tuner_pwa_auth_bootstrap.js'), 'utf8');
const bridge = fs.readFileSync(path.join(repoRoot, 'server/src/assets/ui_tuner_pwa_bridge.js'), 'utf8');
const previewSurface = fs.readFileSync(path.join(repoRoot, 'pc-frontend/src/features/ui-tuner/source-preview/PwaInteractivePreviewSurface.tsx'), 'utf8');
const previewSurfaceCss = fs.readFileSync(path.join(repoRoot, 'pc-frontend/src/features/ui-tuner/source-preview/SourcePreview.module.css'), 'utf8');
const draftContract = fs.readFileSync(path.join(repoRoot, 'pc-frontend/src/features/ui-tuner/source-preview/pwaDesignDraft.ts'), 'utf8');
const designSessionModel = fs.readFileSync(path.join(repoRoot, 'pc-frontend/src/features/ui-tuner/source-preview/pwaDesignSessionModel.ts'), 'utf8');
const designSession = fs.readFileSync(path.join(repoRoot, 'pc-frontend/src/features/ui-tuner/source-preview/usePwaDesignSession.ts'), 'utf8');
const styleInspector = fs.readFileSync(path.join(repoRoot, 'pc-frontend/src/features/ui-tuner/source-preview/PwaStyleInspector.tsx'), 'utf8');

assert.ok(mobileWeb.includes('__UI_TUNER_PWA_BRIDGE_JS__'), 'mobile page should embed the isolated PWA design bridge');
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
assert.ok(bridge.includes("message.type === 'reset-element'"), 'PWA should reset the selected real DOM element');
assert.ok(bridge.includes("message.type === 'apply-draft'"), 'PWA should restore structured page drafts');
assert.ok(bridge.includes("strategy: 'dom-path'"), 'unbound DOM elements should expose an explainable path identity');
assert.ok(bridge.includes('needsBinding: true'), 'generated identities must not pretend to be source bindings');
assert.ok(bridge.includes("'lineHeight'"), 'PWA should support the first manual typography property set');
assert.ok(bridge.includes('let selecting = false'), 'PWA design bridge must default to real interaction');
assert.ok(bridge.includes("mode: 'interact'"), 'PWA ready event must report interaction mode');
assert.ok(bridge.includes("message.type === 'set-session-auth'"), 'preview should accept an ephemeral same-origin session bridge');
assert.ok(bridge.includes("CustomEvent('elon:ui-tuner-session-auth'"), 'session auth must stay in page memory');
assert.ok(!bridge.includes("localStorage.setItem('lodex_token'"), 'preview bridge must not persist the PC token');
assert.ok(bridge.includes("post('route-changed'"), 'PWA should report route changes without parent reloads');
assert.ok(mobileWeb.includes("pageParams.get('ui_tuner_preview') === '1'"), 'session auth listener must be preview-scoped');
assert.ok(designSession.includes("useState<'select' | 'interact'>('interact')"), 'PC PWA canvas must default to real interaction');
assert.ok(designSession.includes("context.post('set-session-auth', { token })"), 'PC should bridge the current same-origin login session');
assert.equal((designSession.match(/window\.addEventListener\('message'/g) || []).length, 1, 'PC session should declare one postMessage listener');
assert.match(designSession, /const bridgeContextRef = useRef\([\s\S]*window\.addEventListener\('message', receive\)[\s\S]*window\.removeEventListener\('message', receive\)\s*\n\s*}, \[\]\)/, 'PC session listener should stay installed while refs provide current render state');
assert.ok(previewSurface.includes('key={reloadKey}'), 'iframe reloads must remain explicit and independent of design mode');
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
assert.ok(designSessionModel.includes('readPwaDesignDraft(project, route)'), 'PC reload should restore the matching project, route, and viewport draft');
assert.ok(designSessionModel.includes('PWA_DESIGN_TRANSACTION_IDLE_MS = 450'), 'continuous controls should be grouped into editing transactions');
assert.ok(designSessionModel.includes('pastDrafts') && designSessionModel.includes('futureDrafts'), 'manual draft editing should support undo and redo');
assert.ok(designSession.includes("model.update('page:clear'"), 'clearing the current page should remain undoable');
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
  filter(callback) { return Array.from(this.values).filter(callback); }
  [Symbol.iterator]() { return this.values[Symbol.iterator](); }
  toggle(value, enabled) {
    if (enabled) this.values.add(value);
    else this.values.delete(value);
  }
}

class FakeStyle {
  constructor() { this.values = new Map(); }
  getPropertyValue(property) { return this.values.get(property) || ''; }
  setProperty(property, value) { this.values.set(property, String(value)); }
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
  }
  appendChild(child) { child.parentElement = this; this.children.push(child); return child; }
  getAttribute(name) {
    if (name === 'id') return this.id || null;
    if (name === 'style') return this.attributes.get(name) ?? null;
    return this.attributes.get(name) ?? null;
  }
  setAttribute(name, value) {
    if (name === 'id') this.id = String(value);
    this.attributes.set(name, String(value));
  }
  removeAttribute(name) { this.attributes.delete(name); }
  closest(selector) {
    if (selector === '[data-component]') return null;
    if (selector.includes('[id]') && this.id) return this;
    return null;
  }
  matches() { return false; }
  getBoundingClientRect() { return { left: 12, top: 24, width: 180, height: 48 }; }
}

function runBridgeBehavior() {
  const parent = { postMessage: (message) => posted.push(message) };
  const posted = [];
  const windowListeners = new Map();
  const documentListeners = new Map();
  const authEvents = [];
  const body = new FakeElement('body');
  const title = new FakeElement('h1', 'topTitle');
  title.childNodes.push({ nodeType: 3, textContent: '一龙项目' });
  body.appendChild(title);
  const document = {
    body,
    styleSheets: [],
    createElement: (tagName) => new FakeElement(tagName),
    addEventListener: (type, listener) => {
      const listeners = documentListeners.get(type) || [];
      listeners.push(listener);
      documentListeners.set(type, listeners);
    },
    querySelectorAll: (selector) => selector === '#topTitle' ? [title] : [],
    querySelector: (selector) => selector === '#topTitle' ? title : null,
  };
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
  };
  const history = {
    pushState() {},
    replaceState() {},
  };
  class FakeCustomEvent {
    constructor(type, options) { this.type = type; this.detail = options.detail; }
  }
  const context = {
    window, document, history, URLSearchParams,
    Node: { TEXT_NODE: 3 }, Element: FakeElement,
    CSS: { escape: (value) => value }, CustomEvent: FakeCustomEvent,
  };
  vm.runInNewContext(bridge, context);

  assert.equal(windowListeners.get('message').length, 1, 'bridge should install exactly one command listener');
  assert.equal(documentListeners.get('click').length, 1, 'bridge should install exactly one selection listener');
  assert.equal(posted.filter((message) => message.type === 'ready').length, 1, 'bridge should emit one ready event');
  assert.equal(posted.filter((message) => message.type === 'route-changed').length, 1, 'bridge should emit one initial route event');

  let interactionPrevented = false;
  documentListeners.get('click')[0]({
    target: title,
    preventDefault() { interactionPrevented = true; },
    stopImmediatePropagation() {},
  });
  assert.equal(interactionPrevented, false, 'the iframe top title must remain clickable in normal interaction mode');
  assert.equal(posted.filter((message) => message.type === 'selection').length, 0, 'normal interaction must not turn a top-title click into a design selection');

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
  const click = { target: title, preventDefault() {}, stopImmediatePropagation() {} };
  documentListeners.get('click')[0](click);
  assert.equal(posted.filter((message) => message.type === 'selection').length, 1, 'the iframe top title must remain clickable and selectable in design mode');

  const messagesBeforeStyle = posted.length;
  command('apply-style', { selector: '#topTitle', style: { fontSize: '22px' } });
  assert.equal(title.style.getPropertyValue('font-size'), '22px', 'font size must update synchronously on the real DOM');
  assert.equal(posted.length, messagesBeforeStyle + 1, 'one style command should produce one acknowledgement without a message loop');
  assert.equal(posted.at(-1).type, 'style-applied');

  const draft = {
    draftKey: 'project-1|/web|||390x844', revision: 7,
    elements: [{ selector: '#topTitle', styleDiff: { fontSize: '24px' } }],
  };
  const acknowledgementsBeforeDraft = posted.filter((message) => message.type === 'draft-applied').length;
  command('apply-draft', draft);
  assert.equal(title.style.getPropertyValue('font-size'), '24px', 'one draft property write must update the selected DOM element');
  assert.equal(posted.filter((message) => message.type === 'draft-applied').length, acknowledgementsBeforeDraft + 1, 'one draft revision should receive one acknowledgement');
  assert.equal(posted.at(-1).payload.revision, 7, 'the acknowledgement should preserve the applied revision');
  command('apply-draft', draft);
  assert.equal(posted.filter((message) => message.type === 'draft-applied').length, acknowledgementsBeforeDraft + 1, 'a repeated draft revision must not apply or acknowledge twice');
}

runAuthBootstrap();
runBridgeBehavior();

console.log('PWA UI tuner bridge tests passed');

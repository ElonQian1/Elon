const assert = require('node:assert');
const fs = require('node:fs');
const path = require('node:path');

const repoRoot = path.resolve(__dirname, '..');
const mobileWeb = fs.readFileSync(path.join(repoRoot, 'server/src/assets/web_page.html'), 'utf8');
const bridge = fs.readFileSync(path.join(repoRoot, 'server/src/assets/ui_tuner_pwa_bridge.js'), 'utf8');
const previewSurface = fs.readFileSync(path.join(repoRoot, 'pc-frontend/src/features/ui-tuner/source-preview/PwaInteractivePreviewSurface.tsx'), 'utf8');

assert.ok(mobileWeb.includes('__UI_TUNER_PWA_BRIDGE_JS__'), 'mobile page should embed the isolated PWA design bridge');
assert.ok(bridge.includes("params.get('ui_tuner_preview') !== '1'"), 'normal PWA pages must not activate the design bridge');
assert.ok(bridge.includes("const SOURCE = 'elon-pwa-design-bridge'"), 'PWA should expose its design bridge to the PC workbench');
assert.ok(bridge.includes("event.origin !== window.location.origin"), 'PWA design bridge must reject cross-origin commands');
assert.ok(bridge.includes("message.type === 'set-mode'"), 'PWA should switch between component selection and normal interaction');
assert.ok(bridge.includes("message.type === 'apply-style'"), 'PWA should apply immediate local style previews');
assert.ok(bridge.includes("message.type === 'reset-styles'"), 'PWA should reset ephemeral preview styles');
assert.ok(bridge.includes('let selecting = false'), 'PWA design bridge must default to real interaction');
assert.ok(bridge.includes("mode: 'interact'"), 'PWA ready event must report interaction mode');
assert.ok(bridge.includes("message.type === 'set-session-auth'"), 'preview should accept an ephemeral same-origin session bridge');
assert.ok(bridge.includes("CustomEvent('elon:ui-tuner-session-auth'"), 'session auth must stay in page memory');
assert.ok(!bridge.includes("localStorage.setItem('lodex_token'"), 'preview bridge must not persist the PC token');
assert.ok(bridge.includes("post('route-changed'"), 'PWA should report route changes without parent reloads');
assert.ok(mobileWeb.includes("pageParams.get('ui_tuner_preview') === '1'"), 'session auth listener must be preview-scoped');
assert.ok(previewSurface.includes("useState<'select' | 'interact'>('interact')"), 'PC PWA canvas must default to real interaction');
assert.ok(previewSurface.includes("post('set-session-auth', { token })"), 'PC should bridge the current same-origin login session');
assert.ok(previewSurface.includes('key={reloadKey}'), 'iframe reloads must remain explicit and independent of design mode');
assert.ok(!bridge.includes("document.addEventListener('pointerover'"), 'PWA selection must not recalculate layout on every mouse hover');
assert.ok(!bridge.includes("String(element.innerText || element.textContent || '')"), 'PWA selection must not read a large container innerText on every click');

console.log('PWA UI tuner bridge tests passed');

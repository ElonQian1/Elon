const assert = require('node:assert');
const fs = require('node:fs');
const path = require('node:path');

const repoRoot = path.resolve(__dirname, '..');
const mobileWeb = fs.readFileSync(path.join(repoRoot, 'server/src/assets/web_page.html'), 'utf8');

assert.ok(mobileWeb.includes("params.get('ui_tuner_preview') !== '1'"), 'normal PWA pages must not activate the design bridge');
assert.ok(mobileWeb.includes("const SOURCE = 'elon-pwa-design-bridge'"), 'PWA should expose its design bridge to the PC workbench');
assert.ok(mobileWeb.includes("event.origin !== window.location.origin"), 'PWA design bridge must reject cross-origin commands');
assert.ok(mobileWeb.includes("message.type === 'set-mode'"), 'PWA should switch between component selection and normal interaction');
assert.ok(mobileWeb.includes("message.type === 'apply-style'"), 'PWA should apply immediate local style previews');
assert.ok(mobileWeb.includes("message.type === 'reset-styles'"), 'PWA should reset ephemeral preview styles');

console.log('PWA UI tuner bridge tests passed');

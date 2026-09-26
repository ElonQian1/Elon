'use strict';

const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');

const root = path.join(__dirname, '..');
const bootstrap = fs.readFileSync(path.join(
  root,
  'desktop-shell',
  'src-tauri',
  'src',
  'local_ai_browser',
  'chatgpt_adapter_bootstrap.rs'
), 'utf8');
const adapter = fs.readFileSync(path.join(
  root,
  'android',
  'app',
  'src',
  'main',
  'assets',
  'chatgpt_web_adapter.js'
), 'utf8');

const observerIndex = bootstrap.indexOf('"chatgpt_web_attachment_transport_observer.js"');
const adapterIndex = bootstrap.indexOf('"chatgpt_web_adapter.js"', observerIndex + 1);
assert.ok(observerIndex >= 0, 'Win bootstrap must include the shared attachment observer');
assert.ok(adapterIndex > observerIndex, 'Win must install the attachment observer before the command adapter');
assert.match(adapter, /attachmentTransportObserver\.arm\(\)/);
assert.ok(Number(/ADAPTER_VERSION: u32 = (\d+)/.exec(bootstrap)?.[1]) >= 207,
  'Attachment observer requires adapter version 207 or newer');

process.stdout.write('PASS Win ChatGPT attachment transport bootstrap\n');

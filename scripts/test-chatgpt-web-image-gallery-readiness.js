'use strict';
const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');
const vm = require('node:vm');
const source = fs.readFileSync(path.join(__dirname,
  '../android/app/src/main/assets/chatgpt_web_image_gallery_sync.js'), 'utf8');

async function run(handles, cached = [], exportOk = true) {
  const events = [];
  let exports = 0;
  const window = {
    __elonChatGptAdapterTargetVersion: 261,
    __elonChatGptCachedImageHandles: cached,
    elonChatGptImageGalleryNative: { postMessage: (json) => events.push(JSON.parse(json).event) },
    __elonChatGptImageAssets: {
      scan: () => handles.map((assetHandle) => ({ assetHandle })),
      request: async () => { exports++; return { ok: exportOk }; }
    },
    setTimeout: (callback) => queueMicrotask(callback)
  };
  vm.runInNewContext(source, { window, location: { origin: 'https://chatgpt.com' },
    document: { querySelectorAll: () => [], scrollingElement: null } });
  for (let i = 0; i < 100; i++) await Promise.resolve();
  return { events, exports };
}

(async () => {
  const empty = await run([]);
  assert.equal(empty.events.at(-1).state, 'failed', 'missing DOM must not cache an empty successful library');
  const handle = 'image_1234567890abcdef';
  const cached = await run([handle], [handle]);
  assert.equal(cached.events.at(-1).state, 'ready');
  assert.equal(cached.exports, 0);
  const loaded = await run([handle]);
  assert.equal(loaded.events.at(-1).state, 'ready');
  assert.equal(loaded.exports, 1);
  const failed = await run([handle], [], false);
  assert.equal(failed.events.at(-1).state, 'failed');
  console.log('IMAGE_GALLERY_READINESS_TESTS=4_passed');
})().catch((error) => { console.error(error); process.exitCode = 1; });

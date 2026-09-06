'use strict';

const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');
const vm = require('node:vm');

const source = fs.readFileSync(path.join(
  __dirname, '..', 'android', 'app', 'src', 'main', 'assets',
  'chatgpt_web_image_gallery_sync.js'
), 'utf8');

async function runScenario(requestResult) {
  const events = [];
  const requests = [];
  let scans = 0;
  let complete;
  const completion = new Promise(resolve => { complete = resolve; });
  const document = {
    querySelectorAll: () => [],
    scrollingElement: { scrollHeight: 2000, clientHeight: 600, scrollTo() {} }
  };
  const window = {
    document,
    setTimeout: (callback) => setTimeout(callback, 0),
    elonChatGptImageGalleryNative: {
      postMessage: (payload) => {
        const event = JSON.parse(payload).event;
        events.push(event);
        if (event.type === 'image_gallery_snapshot' && event.state !== 'loading') complete();
      }
    },
    __elonChatGptAdapterTargetVersion: 208,
    __elonChatGptCachedImageHandles: ['image_0000000000000001'],
    __elonChatGptImageAssets: {
      scan: () => {
        scans += 1;
        if (scans < 5) return [];
        return [1, 2, 3].map((value) => ({
          assetHandle: `image_${String(value).padStart(16, '0')}`
        }));
      },
      request: async (handle) => {
        requests.push(handle);
        return { ok: requestResult };
      }
    }
  };
  window.window = window;

  vm.runInNewContext(source, {
    window,
    document,
    location: { origin: 'https://chatgpt.com' },
    Promise,
    Set,
    Array,
    JSON,
    Number,
    String,
    Error
  }, { filename: 'chatgpt_web_image_gallery_sync.js' });

  let deadline;
  try {
    await Promise.race([completion, new Promise((_, reject) => {
      deadline = setTimeout(() => reject(new Error('gallery_completion_timeout')), 5000);
    })]);
  } finally { clearTimeout(deadline); }
  return { events, requests, scans };
}

(async () => {
  const success = await runScenario(true);
  assert.ok(success.scans >= 5, 'an initially empty DOM must keep waiting');
  assert.deepEqual(success.requests, [
    'image_0000000000000002',
    'image_0000000000000003'
  ]);
  assert.equal(success.events[0].state, 'loading');
  assert.equal(success.events.at(-1).state, 'ready');
  assert.equal(success.events.at(-1).observedCount, 3);

  const failure = await runScenario(false);
  assert.equal(failure.events.at(-1).state, 'failed');
  console.log('chatgpt web image gallery sync tests passed');
})().catch((error) => {
  console.error(error);
  process.exitCode = 1;
});

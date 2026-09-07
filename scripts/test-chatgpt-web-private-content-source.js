'use strict';
const { test } = require('node:test');
const assert = require('node:assert/strict');
const fs = require('node:fs');
const vm = require('node:vm');
const content = require('../android/app/src/main/assets/chatgpt_web_private_content_source.js');
const library = require('../android/app/src/main/assets/chatgpt_web_private_library_download.js');

test('shared content policy retains the released download URL boundary', () => {
  for (const path of ['/api/estuary/content', '/backend-api/estuary/content']) {
    for (const prefix of ['', 'https://chatgpt.com', 'https://chatgpt.com:443']) {
      const value = prefix + path + '?id=file-synthetic&sig=synthetic';
      assert.equal(content.contentUrl(value), 'https://chatgpt.com' + path + '?id=file-synthetic&sig=synthetic');
      assert.equal(library.contentUrl(value), content.contentUrl(value));
    }
  }
  for (const value of [null, {}, '/api/other', '//chatgpt.com/api/estuary/content',
    'https://external.test/api/estuary/content', '/api/estuary/content#fragment',
    '/api/estuary/content\n?x=y', '/api/estuary/content?x=' + 'x'.repeat(16384)]) {
    assert.equal(content.contentUrl(value), null);
    assert.equal(library.contentUrl(value), null);
  }
});

test('production assembly loads one content policy before image and binary consumers', () => {
  const adapter = require('./chatgpt-web-adapter-assembly').readAdapterSource();
  const names = [...adapter.matchAll(/"(chatgpt_web_[A-Za-z0-9_]+\.js)"/g)].map(match => match[1]);
  const name = 'chatgpt_web_private_content_source.js';
  assert.equal(names.filter(value => value === name).length, 1);
  assert.ok(names.indexOf(name) < names.indexOf('chatgpt_web_image_assets.js'));
  assert.ok(names.indexOf(name) < names.indexOf('chatgpt_web_private_library_download.js'));
  const window = { location: { origin: 'https://chatgpt.com' } };
  const context = vm.createContext({ window, URL });
  for (const file of [name, 'chatgpt_web_private_library_download.js']) {
    vm.runInContext(fs.readFileSync('android/app/src/main/assets/' + file, 'utf8'), context);
  }
  assert.equal(window.__elonChatGptPrivateLibraryDownload.contentUrl('/api/estuary/content'),
    'https://chatgpt.com/api/estuary/content');
});

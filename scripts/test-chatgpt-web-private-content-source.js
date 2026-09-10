'use strict';
const { test } = require('node:test');
const assert = require('node:assert/strict');
const fs = require('node:fs');
const vm = require('node:vm');
const content = require('../android/app/src/main/assets/chatgpt_web_private_content_source.js');
const library = require('../android/app/src/main/assets/chatgpt_web_private_library_download.js');

test('download diagnostics preserve only source categories and fixed route words', () => {
  for (const [url, origin, path] of [
    ['/backend-api/estuary/content?id=secret&sig=credential', 'same_origin', '/backend-api/estuary/content'],
    ['https://chatgpt.com/api/library/files/libfile_secret/download?secret=value', 'same_origin', '/api/library/files/{id}/download'],
    ['https://chatgpt.com/private-name/file-private', 'same_origin', '/{id}/{id}'],
    ['https://bucket.oaiusercontent.com/secret?sig=credential', 'oaiusercontent', ''],
    ['https://bucket.blob.core.windows.net/private?sig=credential', 'azure_blob', ''],
    ['https://private-host.test/private', 'other_https', ''],
    ['data:text/plain,private', 'non_https', ''],
    ['https://chatgpt.com/', 'same_origin', ''],
  ]) {
    const value = content.describe(url);
    assert.equal(value.origin, origin); assert.equal(value.path, path);
    assert.doesNotMatch(JSON.stringify(value), /secret|"credential"|private|bucket/);
    assert.deepEqual(Object.keys(value), ['schema', 'observed', 'origin', 'path', 'relative', 'whitespace', 'credentials', 'port', 'fragment']);
  }
  const value = content.describe(' https://user:pass@chatgpt.com:8443/api/content#private');
  assert.equal(value.whitespace, true); assert.equal(value.credentials, true);
  assert.equal(value.port, true); assert.equal(value.fragment, true);
  for (const input of [null, {}, '', 'https://[', 'x'.repeat(16385)]) {
    assert.doesNotThrow(() => content.describe(input));
  }
});

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

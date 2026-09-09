'use strict';
const assert = require('node:assert/strict');
const { test } = require('node:test');
const { runtimeAssets: snapshot } = require('../android/app/src/main/assets/chatgpt_web_private_protocol_evidence.js');

function page(resources = [], nodes = []) {
  return {
    location: { origin: 'https://chatgpt.com' },
    performance: { getEntriesByType: (type) => {
      assert.equal(type, 'resource'); return resources.map(name => ({ name }));
    } },
    document: { querySelectorAll: (selector) => {
      assert.equal(selector, 'link[rel="modulepreload"],script[src]'); return nodes;
    } },
    fetch: () => assert.fail('diagnostic must not make requests'),
  };
}

test('only exact public asset filenames are returned without content, query or headers', () => {
  const p = page([
    'https://chatgpt.com/cdn/assets/shared-abc123.js',
    'https://chatgpt.com:443/cdn/assets/shared-abc123.js',
    'https://chatgpt.com/cdn/assets/private.js?token=secret',
    'https://chatgpt.com/cdn/assets/private.js#secret',
    'https://chatgpt.com/cdn/assets/nested/private.js',
    'https://chatgpt.com/cdn/assets/%73ecret.js',
    'https://chatgpt.com:444/cdn/assets/private.js',
    'https://secret@chatgpt.com/cdn/assets/private.js',
    'https://other.test/cdn/assets/private.js',
    'https://chatgpt.com/backend-api/conversation/private',
    'https://chatgpt.com/cdn/assets/back\\slash.js',
    'https://chatgpt.com/cdn/assets/line\nfeed.js',
  ], [{ href: '/cdn/assets/conversation-abc123.js' }, { src: '/cdn/assets/shared-abc123.js' }]);
  Object.defineProperty(p.document, 'body', { get: () => assert.fail('no page text') });
  const result = JSON.parse(snapshot(p));
  assert.deepEqual(result, { schema: 'elon.private_runtime_assets.v1',
    assets: ['conversation-abc123.js', 'shared-abc123.js'], truncated: false });
  assert.doesNotMatch(JSON.stringify(result), /secret|token|https|private\.js/);
});

test('bounded deduplicated snapshot explicitly reports incomplete inventory', () => {
  const p = page(Array.from({ length: 120 }, (_, n) => `/cdn/assets/chunk-${n}.js`));
  const result = JSON.parse(snapshot(p));
  assert.equal(result.assets.length, 96);
  assert.equal(result.truncated, true);
  assert.ok(JSON.stringify(result).length < 12000);
});

test('late runtime roles survive inventory overflow without raising the bound', () => {
  const roles = ['c2675c8c-build1.js', '4813494d-build1.js',
    'conversation-small-build1.js', '8b34dbc2-build1.js', '2340486e-build1.js',
    'c2675c8c-build2.js'];
  const p = page(Array.from({ length: 200 }, (_, n) => `/cdn/assets/chunk-${n}.js`),
    roles.map(name => ({ href: '/cdn/assets/' + name })));
  const result = JSON.parse(snapshot(p));
  assert.equal(result.assets.length, 96);
  assert.equal(result.truncated, true);
  for (const name of roles) assert.ok(result.assets.includes(name), name);
});

test('wrong origin is rejected and partial observation never implies unavailable capability', () => {
  assert.equal(snapshot({ location: { origin: 'https://other.test' } }), null);
  const p = page([], [{ src: '/cdn/assets/known.js' }]);
  p.performance.getEntriesByType = () => { throw new Error('unavailable'); };
  assert.deepEqual(JSON.parse(snapshot(p)), {
    schema: 'elon.private_runtime_assets.v1', assets: ['known.js'], truncated: true,
  });
  assert.deepEqual(JSON.parse(snapshot(page())).assets, []);
});

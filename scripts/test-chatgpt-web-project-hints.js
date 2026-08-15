'use strict';

const assert = require('assert');
const hints = require('../android/app/src/main/assets/chatgpt_web_adapter_project_hints.js');

const cached = {
  id: 'g-p-cached',
  title: 'Cached project',
  path: '/g/g-p-cached/project'
};
const observed = {
  id: 'g-p-observed',
  title: 'Observed project',
  path: '/g/g-p-observed/project',
  active: true
};

assert.deepStrictEqual(hints.sanitize([
  cached,
  cached,
  { id: 'wrong', title: 'Rejected', path: '/g/g-p-rejected/project' },
  { id: 'g-p-cross', title: 'Rejected', path: 'https://example.com/g/g-p-cross/project' }
]), [Object.assign({ active: false }, cached)]);

assert.deepStrictEqual(hints.merge([observed], [cached]).map((project) => project.id), [
  'g-p-cached',
  'g-p-observed'
]);

assert.deepStrictEqual(hints.missingTitles([
  'Cached project',
  'New project',
  ' new   project ',
  ''
], [cached]), ['New project']);

process.stdout.write('chatgpt project hint policy tests passed\n');

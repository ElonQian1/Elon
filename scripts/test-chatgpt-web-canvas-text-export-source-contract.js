'use strict';
const test = require('node:test'), assert = require('node:assert/strict');
const fs = require('node:fs'), path = require('node:path'), crypto = require('node:crypto'), vm = require('node:vm');
const text = require('../android/app/src/main/assets/chatgpt_web_private_canvas_text_export.js');
const { parseSource } = require('./analyze-chatgpt-runtime-contracts.cjs');
const hashes = {
  'bc86e6a9-lyyfjzfq9uy5wnj2.js': '9417a623b117d5c76343b91f9a2c6c3a0402c24d50cec8fd8a671e2fc1493eeb',
  'conversation-small-h1dtzoris1y9588z.js': 'da08c64c132306779e09ba89cac64fa560b120e7560ffdc29b3ce5a0b8ccd67e',
  '4813494d-gf2h57w5fiay19bd.js': '6c015001732054f4143ef1922609407c540967762109dcd128bbf56706889c3e',
  'ac827dee-b2d6yzv273npzjvz.js': '1124a81c8fb3dcc4042b33666f47af733fe9e0dd2d8a9dd607a1f0bccee34d4d',
  'e5d54aa7-o5mtxxnk4j8zox9y.js': '393bf8650d692d62ffdf1d5522b71d49bd09a783b81046112c911bf09f4c82d0',
  '1c4de3ec-ix5n1yyu8whxeib8.js': '7f5583294c72209a9bb06eb870cc021bc52862606e9f083e48a0048f65007296',
  '6afb0137-dqge0sx8jli56ai8.js': '33dfe354d1cd48931719cdd38e93aff36ecfead535e6a4b7c812e7f33f59d177'
};
const optional = { skip: !process.env.CHATGPT_PUBLIC_RUNTIME_DIR && 'Reviewed public sources are optional offline inputs.' };

test('source format table and Markdown serializer match the reviewed official module, not inferred endpoints', optional, () => {
  const sources = new Map();
  for (const [file, expected] of Object.entries(hashes)) {
    const source = fs.readFileSync(path.join(process.env.CHATGPT_PUBLIC_RUNTIME_DIR, file), 'utf8');
    assert.equal(crypto.createHash('sha256').update(source).digest('hex'), expected, file);
    sources.set(file, source);
  }
  const exportSource = parseSource(sources.get('bc86e6a9-lyyfjzfq9uy5wnj2.js'));
  const conversation = parseSource(sources.get('conversation-small-h1dtzoris1y9588z.js'));
  const definition = (source, name) => {
    const nodes = source.definitions.get(name); assert.equal(nodes.length, 1, name);
    return source.text.slice(nodes[0].start, nodes[0].end);
  };
  // Only hash-pinned enum/table expressions run here, never the full application or authenticated runtime.
  const enumValue = vm.runInNewContext('(' + definition(conversation, conversation.exported.get('kbt')) + ')', {}, { timeout: 1000 });
  const table = vm.runInNewContext('(' + definition(exportSource, 'Z') + ')', { A: enumValue }, { timeout: 1000 });
  const sourceTypes = Object.fromEntries(Object.entries(table).filter(([key]) => key.startsWith('code/')).map(([key, info]) =>
    [key, { extension: info.filenameExtension, mediaType: info.blobType }]));
  assert.deepEqual(text.sourceTypes, sourceTypes);
  assert.deepEqual(text.describe('document', 'md'), { extension: table.document.filenameExtension, mediaType: table.document.blobType });
  assert.equal(text.describe('loading', 'source'), null);

  const g = definition(exportSource, 'G'), hook = definition(exportSource, 'Xe');
  assert.ok(g.includes('if(Ne.every(t=>!e.includes(t)))return e'));
  assert.ok(g.includes('o.use(a).use(i,{preserve:t?.preserveContentReferences?[f]:void 0}).use(r)'));
  assert.ok(g.includes('return String(s).trim()'));
  assert.ok(hook.includes('r=await G(e.content)')); // Markdown download does not request preserveContentReferences.
  assert.ok(hook.includes('new Blob([r],{type:n})'));
  assert.ok(hook.includes('new Blob([e.content],{type:n})'));
  assert.ok(![...exportSource.exported.values()].includes('G')); // Import actual plugins, not a guessed callable alias.

  const implementation = fs.readFileSync(path.join(__dirname, '../android/app/src/main/assets/chatgpt_web_private_canvas_text_export.js'), 'utf8');
  assert.ok(implementation.includes("PROFILE = 'web_20260912'"));
  for (const file of ['conversation-small-h1dtzoris1y9588z.js', 'e5d54aa7-o5mtxxnk4j8zox9y.js',
    '1c4de3ec-ix5n1yyu8whxeib8.js', '6afb0137-dqge0sx8jli56ai8.js']) assert.ok(g.includes(file) && implementation.includes(file));
  for (const alias of ['IAn', 'LAn']) assert.ok(conversation.exported.has(alias));
  for (const [file, init, namespace, publicName] of [
    ['e5d54aa7-o5mtxxnk4j8zox9y.js', 'n', 'r', 'CANVAS_REMARK_PLUGINS'],
    ['1c4de3ec-ix5n1yyu8whxeib8.js', 't', 'r', 'stripDirectivePlugin'],
    ['6afb0137-dqge0sx8jli56ai8.js', 'i', 'r', 'hiveLogDirectivePlugin']
  ]) {
    const parsed = parseSource(sources.get(file));
    assert.ok(parsed.exported.has(init) && parsed.exported.has(namespace)); assert.ok(parsed.text.includes(publicName));
  }
  assert.ok(sources.get('ac827dee-b2d6yzv273npzjvz.js').includes('s=[t,o]'));
  const shared = parseSource(sources.get('4813494d-gf2h57w5fiay19bd.js'));
  assert.equal(definition(shared, shared.exported.get('dQ')), '`contentReference`');
  assert.ok(sources.get('6afb0137-dqge0sx8jli56ai8.js').includes('l=`hiveTranscript`'));
});

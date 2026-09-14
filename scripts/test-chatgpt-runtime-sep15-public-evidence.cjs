'use strict';
const test = require('node:test');
const assert = require('node:assert/strict');
const fs = require('node:fs'), path = require('node:path'), crypto = require('node:crypto');
const { parseSource, roleFiles } = require('./analyze-chatgpt-runtime-contracts.cjs');
const profile = require('./fixtures/chatgpt-runtime-bindings-sep15.cjs');
const directory = process.env.CHATGPT_PUBLIC_RUNTIME_DIR;
const hashes = {
  anchor: '1b1a03a6f37c5f65f6e2405b77d29f0c394d4d4669cd70dde08ff2752bb59324',
  shared: '26a355cdf5b2463576675472901d4ea1d974765971208cfacdfb0aa853c0eddf',
  conversation: '80f338297f77ccfe91b2ee86c22888606da5c662bbb3ad09c81f1867173952c7',
  composer: '852e5603f1d10542e8752f72d7a864cfa2442d8f3f46a8a3c9069a0c93a006ca',
  react: 'a0e3af9dc19a43aff7b55fe57a6ebca9f8ce108eb599880091abbec880551e9d'
};

test('Sep 15 actual namespace contracts and changed history/attachment semantics', {
  skip: !directory && 'Set CHATGPT_PUBLIC_RUNTIME_DIR to the reviewed public assets.'
}, () => {
  const modules = {};
  for (const [role, file] of Object.entries({ anchor: profile.anchor, ...profile.files })) {
    const bytes = fs.readFileSync(path.join(directory, file));
    assert.equal(crypto.createHash('sha256').update(bytes).digest('hex'), hashes[role]);
    modules[role] = parseSource(bytes.toString('utf8'));
  }
  assert.deepEqual(roleFiles(modules.anchor.sources, true), profile.files);
  for (const [role, mappings] of Object.entries(profile.expectedExports))
    for (const [name, exported] of Object.entries(mappings))
      assert.ok(modules[role].exported.has(exported), role + '.' + name);
  const definition = (role, name, local = false) => {
    const module = modules[role], nodes = module.definitions.get(local ? name : module.exported.get(name));
    assert.equal(nodes?.length, 1, role + '.' + name);
    return module.text.slice(nodes[0].start, nodes[0].end);
  };
  const stop = definition('conversation', 'fGt');
  assert.match(stop, /wd\(/);
  assert.deepEqual(modules.conversation.imports.get('wd'), { file: './' + profile.files.shared, name: 'wC' });
  assert.match(stop, /safePost\(`\/stop_conversation`/);
  assert.equal(definition('shared', 'OK'), 'function jw(){return rg(Aw)}');
  assert.match(definition('shared', 'Aw', true), /accountKey===kw\(\)\?e.pin:null/);
  assert.match(definition('shared', 'nX'), /startsWith\(El\)/);
  assert.equal(definition('shared', 'El', true), '`WEB:`');
  assert.equal(definition('shared', 'VK'), 'function cw(){return rg(Cw)}');
  assert.equal(definition('shared', 'Cw', true), 'J(()=>sw(Sw().data))');
  assert.match(definition('shared', 'sw', true), /accountItems.find\(t=>t.id===e.currentAccountId\)/);
  const ordinaryScope = definition('shared', '$2', true);
  assert.ok(ordinaryScope.includes('l?.contextScopes?.some(e=>e!==`GLOBAL`)'));
  assert.ok(ordinaryScope.includes('s.context_scopes?.some(e=>e!==`GLOBAL`)'));
  const hydration = [...modules.conversation.definitions.values()].flat().map(node =>
    modules.conversation.text.slice(node.start, node.end));
  assert.ok(hydration.some(body => body.includes('j=t.context_scopes,M=j&&j.length>0?Array.from(j):void 0') &&
    body.includes('contextScopes:y==null?M??n?.contextScopes??null:M??null')));
  assert.equal(definition('conversation', 'tOt'), 'zd(()=>v$n())');
  assert.match(definition('conversation', 'v$n', true), /cleanTtlMs/);
  assert.match(definition('conversation', 'v$n', true), /hydrateSessionFromLibrary/);

  const history = definition('conversation', 'VOn');
  assert.match(history, /shouldApplyResponse:l/);
  assert.match(history, /pendingDrafts.length>0\?JQe/);
  assert.match(history, /O&&_!=null&&g!=null&&_\$e/);
  const reconcile = definition('conversation', '_$e', true);
  assert.match(reconcile, /a\?\.aborted\|\|o\?\.\(\)===!1/);
  assert.match(reconcile, /safeGet\(`\/conversation\/\{conversation_id\}\/messages\/\{message_id\}\/existence`/);
  assert.equal(reconcile.includes('safePost'), false, 'draft reconciliation reads; it does not resend');
  const applied = definition('conversation', 'QQe', true);
  assert.match(applied, /r.userId!==e.userId\|\|t.conversation_id!==e.serverThreadId/);
  assert.match(applied, /s===`present`/);
  assert.equal(applied.includes('safePost'), false);
  const attachments = definition('conversation', 'zmt');
  assert.match(attachments, /^function p_r\(e,t,n,r,i,a\)/);
  assert.match(attachments, /let o=V6t\(a\?\.mountedLibraryFileId\)\?a:null,s=o==null\?a:null/);
  assert.match(attachments, /o!=null&&g&&o.fileId.length>0/);
  assert.match(attachments, /!c.some\(e=>e.mountedLibraryFileId===o.mountedLibraryFileId\)/);
  assert.match(attachments, /mounted_library_mime_type:o.mimeType/);
  assert.match(attachments, /Rhe\(n,r,i\)/);
  assert.deepEqual(modules.conversation.imports.get('Rhe'), { file: './' + profile.files.shared, name: 'im' });
  assert.match(definition('shared', 'im'), /lX\(e,t\)/);
  assert.match(definition('shared', 'am'), /e\?\.product_features/);
  const editor = definition('composer', 'Fh');
  assert.match(editor, /validateChatAttachment:\(e,t,n,r,i,a\)=>OB.validateChatAttachmentSize\(e,t\)/);
  assert.match(editor, /handleChatLibraryFileCount\(e,n,r,i,a\)/);
  assert.ok(definition('composer', profile.temporary.owner, true).includes(profile.temporary.action));
  assert.ok(modules.composer.definitions.has(profile.toolOwner));

  const react = modules.react;
  const ast = require(process.env.CHATGPT_AST_PARSER || 'acorn').parse(react.text,
    { ecmaVersion: 'latest', sourceType: 'module' });
  const top = ast.body.filter(node => node.type === 'VariableDeclaration').flatMap(node => node.declarations);
  for (const [exported, factory] of [['Rn', 'i'], ['Ut', 'kS'], ['Ht', 'jS']]) {
    const nodes = top.filter(node => node.id.name === react.exported.get(exported) && node.init);
    assert.equal(nodes.length, 1);
    assert.ok(react.text.slice(nodes[0].start, nodes[0].end).includes('t.exports=' + factory + '()'));
  }
  assert.equal(react.exported.get('kn'), 'Mn');
  assert.match(definition('react', 'Fn'), /Mn=dn/);
});

test('all 100 Sep 15 canonical exports resolve through the production loader', async () => {
  const binding = require('../android/app/src/main/assets/chatgpt_web_private_runtime_bindings');
  const cdn = 'https://chatgpt.com/cdn/assets/';
  const page = { location: { origin: 'https://chatgpt.com' }, document: {}, setTimeout, clearTimeout,
    __elonChatGptDocumentToken: 'doc_fixture', performance: { getEntriesByName: url => url === cdn + profile.anchor ? [{}] : [] } };
  const namespaces = Object.fromEntries(Object.entries(profile.expectedExports).map(([role, map]) =>
    [role, Object.fromEntries(Object.entries(map).map(([name, exported]) => [exported, { name }]))]));
  const runtime = binding.create(page, { loadRuntime: async url => namespaces[
    Object.keys(profile.files).find(role => url === cdn + profile.files[role])] });
  let count = 0;
  for (const [role, mappings] of Object.entries(profile.expectedExports)) {
    const exposed = await runtime.load(role);
    assert.deepEqual(Object.keys(exposed).sort(), Object.keys(mappings).sort());
    for (const [name, exported] of Object.entries(mappings)) { count++; assert.equal(exposed[name], namespaces[role][exported]); }
  }
  assert.equal(count, 100);
  const react = runtime.peek('react');
  namespaces.react.kn = { initialized: true };
  assert.equal(react.intlProvider, namespaces.react.kn, 'lazy provider is a live binding');
  assert.equal(runtime.state().profile_id, profile.id);
});

test('Sep 15 Canvas export, generation and restore payload use reviewed public contracts', {
  skip: !directory && 'Set CHATGPT_PUBLIC_RUNTIME_DIR to the reviewed public assets.'
}, () => {
  const files = {
    'bc86e6a9-hff15k68zj00cjy7.js': '9503acdccf51c2b2be2e41f78cdc111794841d4eaaaecb13f6b599504cf17975',
    '6afb0137-g283lmq2pqmv3jkl.js': '1309b67cbe5a68c17ab9bc312088e61d5c337231269bea12dcc214604c43360b',
    'd3304073-nglhmqv6gfc20nrf.js': '537254e370736bfc54d583fe3c199bab9f368af040b58b1692bbde04e8a03cb6',
    'e5d54aa7-hupdur95y35b5iac.js': '08f3e9af0fd1eacbbd5940301c1f15a4d071c810796341849460a5f101bc194a',
    '1c4de3ec-n5z38kp7mdcow2vj.js': '591eca28c73b4e17512128137dfb50a9b3036a475d064655b0de67735ad48fda',
    [profile.files.conversation]: hashes.conversation
  };
  const modules = {};
  for (const [file, hash] of Object.entries(files)) {
    const bytes = fs.readFileSync(path.join(directory, file));
    assert.equal(crypto.createHash('sha256').update(bytes).digest('hex'), hash);
    modules[file] = parseSource(bytes.toString('utf8'));
  }
  const exporter = modules['bc86e6a9-hff15k68zj00cjy7.js'].text;
  assert.match(exporter, /e\.BMn\(\),e\.VMn/);
  assert.match(exporter, /CANVAS_REMARK_PLUGINS/);
  assert.match(exporter, /stripDirectivePlugin/);
  assert.match(exporter, /hiveLogDirectivePlugin/);
  for (const [file, exports] of Object.entries({
    [profile.files.conversation]: ['BMn', 'VMn'],
    'e5d54aa7-hupdur95y35b5iac.js': ['n', 'r'],
    '1c4de3ec-n5z38kp7mdcow2vj.js': ['t', 'r'],
    '6afb0137-g283lmq2pqmv3jkl.js': ['i', 'r'],
    'd3304073-nglhmqv6gfc20nrf.js': ['i', 'a']
  })) for (const name of exports) assert.ok(modules[file].exported.has(name), file + '.' + name);
  const conversation = modules[profile.files.conversation], restore = conversation.definitions.get('Ror');
  assert.equal(restore?.length, 1);
  const body = conversation.text.slice(restore[0].start, restore[0].end);
  assert.match(body, /textdocId:e,versionInt:t,restoreFromVersionInt:n/);
  assert.match(body, /safePost\(`\/textdoc\/\{textdoc_id\}\/restore`/);
  assert.match(body, /requestBody:\{version:t,restore_from_version:n\}/);
});

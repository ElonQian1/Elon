'use strict';
const test = require('node:test');
const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');
const crypto = require('node:crypto');
const { parseSource, roleFiles } = require('./analyze-chatgpt-runtime-contracts.cjs');
const profile = require('./fixtures/chatgpt-runtime-bindings-sep12');
const directory = process.env.CHATGPT_PUBLIC_RUNTIME_DIR;
const hashes = {
  anchor: '626dec60a232340efeac2f154db07232740de506b221abf5fafe0e71d4ab03a2',
  shared: '6c015001732054f4143ef1922609407c540967762109dcd128bbf56706889c3e',
  conversation: 'da08c64c132306779e09ba89cac64fa560b120e7560ffdc29b3ce5a0b8ccd67e',
  composer: '1d0b132fe9b13120370395bbfbe4bf3324c4dc1213b80e591cd3c6a6db7d16a1'
};

test('reviewed public modules contain actual exports, not coincidentally named imports', {
  skip: !directory && 'Set CHATGPT_PUBLIC_RUNTIME_DIR to the reviewed public assets; no network in tests.'
}, () => {
  const parsed = {};
  for (const [role, file] of Object.entries({ anchor: profile.anchor, ...profile.files })) {
    const source = fs.readFileSync(path.join(directory, file));
    assert.equal(crypto.createHash('sha256').update(source).digest('hex'), hashes[role]);
    // Parse only. Importing public modules here could initialize unrelated website code.
    parsed[role] = parseSource(source.toString('utf8'));
  }
  assert.deepEqual(roleFiles(parsed.anchor.sources), profile.files);
  for (const role of Object.keys(profile.files)) {
    for (const [canonical, exported] of Object.entries({ ...profile.expectedExports[role], ...profile.extraExports[role] })) {
      assert.ok(parsed[role].exported.has(exported), role + '.' + canonical + ' must be exported');
    }
  }
  const conversation = parsed.conversation;
  assert.equal(conversation.exported.has('one'), false);
  assert.equal(conversation.imports.get('one').file, './' + profile.files.shared);
  assert.equal(conversation.exported.has('dir'), false);
  assert.equal(profile.extraExports.conversation.canvasEdits, undefined);
  const stop = conversation.definitions.get(conversation.exported.get(profile.expectedExports.conversation.FVt))[0];
  const body = conversation.text.slice(stop.start, stop.end);
  assert.match(body, /Uo\(c\)/);
  assert.deepEqual(conversation.imports.get('Uo'), { file: './' + profile.files.shared, name: 'aC' });
  assert.deepEqual(conversation.imports.get('yn'), { file: './' + profile.files.shared, name: 'GJ' });
  assert.deepEqual(conversation.imports.get('au'), { file: './' + profile.files.shared, name: 'rtt' });
  assert.match(body, /safePost\(`\/stop_conversation`/);
  assert.ok(parsed.composer.definitions.has(profile.toolOwner));
  const temporary = parsed.composer.definitions.get(profile.temporary.owner)[0];
  assert.ok(parsed.composer.text.slice(temporary.start, temporary.end).includes(profile.temporary.action));

  const inspect = (file, hash) => {
    const source = fs.readFileSync(path.join(directory, file));
    assert.equal(crypto.createHash('sha256').update(source).digest('hex'), hash);
    return parseSource(source.toString('utf8'));
  };
  const react = inspect('2340486e-dyt4epctwx2pn2sj.js', 'bd1f145733f12933c92dd18fbb8e982601c65ef22a41dd2f898fc8f357857261');
  const editor = inspect('ac476d6c-foq8estmw3bbr7op.js', 'f341cbd1ebc80c14838f6bada429d1826a5a78577f094f5be452ca7663c3c895');
  const definition = (module, name) => {
    const nodes = module.definitions.get(module.exported.get(name));
    assert.equal(nodes?.length, 1, name + ' must resolve to one definition');
    return module.text.slice(nodes[0].start, nodes[0].end);
  };
  assert.match(definition(conversation, 'Avt'), /dir\(\(\{timestamps:t\}\)/);
  assert.ok(definition(conversation, 'Dvt').includes('dir=at(Eu(()=>({userEdits:{},timestamps:{}})))'));
  assert.deepEqual(editor.imports.get('Le'), { file: './' + profile.files.conversation, name: 'Avt' });
  assert.deepEqual(editor.imports.get('We'), { file: './' + profile.files.conversation, name: 'Dvt' });
  assert.match(editor.text, /mutationKey:\[`canvas`,`textdoc`,`persist`\],mutationFn:Da/);
  assert.match(editor.text, /Da=async\(\{lastVersion:e,textdocId:t,content:n,comments:r\}\)/);
  // Single-letter names are reused in nested functions. Match top-level declarations,
  // not the advisory fingerprint index's scope-insensitive assignment collection.
  const ast = require(process.env.CHATGPT_AST_PARSER || 'acorn').parse(react.text,
    { ecmaVersion: 'latest', sourceType: 'module' });
  const declarations = ast.body.filter(node => node.type === 'VariableDeclaration').flatMap(node => node.declarations);
  for (const [exported, factory] of [['zn', 'i'], ['Wt', 'kS'], ['Ut', 'jS']]) {
    const matches = declarations.filter(node => node.id.name === react.exported.get(exported) && node.init);
    assert.equal(matches.length, 1);
    assert.ok(react.text.slice(matches[0].init.start, matches[0].init.end).includes('t.exports=' + factory + '()'));
  }
});

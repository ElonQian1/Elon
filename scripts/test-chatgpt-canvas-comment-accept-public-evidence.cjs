'use strict';
const test = require('node:test'), assert = require('node:assert/strict');
const fs = require('node:fs'), path = require('node:path'), crypto = require('node:crypto');
const { parseSource } = require('./analyze-chatgpt-runtime-contracts.cjs');
const directory = process.env.CHATGPT_PUBLIC_RUNTIME_DIR;
test('official comment acceptance awaits the versioned DELETE before the captured Canvas edit callback', {
  skip: !directory && 'Requires reviewed local public assets; never executes the downloaded modules.'
}, () => {
  const read = (file, hash) => {
    const bytes = fs.readFileSync(path.join(directory, file));
    assert.equal(crypto.createHash('sha256').update(bytes).digest('hex'), hash);
    return parseSource(bytes.toString('utf8'));
  };
  const editor = read('ac476d6c-foq8estmw3bbr7op.js', 'f341cbd1ebc80c14838f6bada429d1826a5a78577f094f5be452ca7663c3c895');
  const conversation = read('conversation-small-h1dtzoris1y9588z.js', 'da08c64c132306779e09ba89cac64fa560b120e7560ffdc29b3ce5a0b8ccd67e');
  const source = (module, local) => {
    const nodes = module.definitions.get(local); assert.equal(nodes?.length, 1);
    return module.text.slice(nodes[0].start, nodes[0].end);
  };
  assert.deepEqual(editor.imports.get('Ve'), { file: './conversation-small-h1dtzoris1y9588z.js', name: 'Cvt' });
  assert.deepEqual(editor.imports.get('kr'), { file: './d3304073-k8khdx5ezvb9oyu8.js', name: 'a' });
  const main = source(editor, 'Ha');
  assert.ok(main.includes('k=kr(s,D)'));
  assert.ok(main.includes('let{id:n,at:r,content:i}=t;if(await Xe(n,xt.ACCEPT)===!1)return Je(!1);k({sourceEvent:e,content:i,userMessageType:Ie.ACCEPT_COMMENT,sourceRange:r,action:hr.EDIT,selectionMetadata:{selection_type:gr.SELECTION,selection_position_range:r}})'));
  assert.equal(conversation.exported.get('Cvt'), 'Iir');
  const hook = source(conversation, 'Iir');
  assert.ok(hook.includes('i=e?.versionInt')); assert.ok(hook.includes('await kir({lastVersion:i,textdocId:a,commentId:t,reason:n})'));
  assert.ok(hook.includes('versionInt:e')); assert.ok(hook.includes('return!1'));
  const api = source(conversation, 'kir');
  assert.ok(api.includes('safeDelete(`/textdoc/{textdoc_id}/{version}/comment/{comment_id}`'));
  assert.ok(api.includes('query:{reason:r},path:{textdoc_id:t,version:String(e),comment_id:n}'));
  assert.ok(api.endsWith('.version'));
  assert.match(source(conversation, 'T7n'), /ACCEPT=`accept`,e.DISMISS=`dismiss`/);
  assert.match(source(conversation, 'O7n'), /ACCEPT_COMMENT=`accept_comment`/);
});

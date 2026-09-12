'use strict';
const test = require('node:test');
const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');
const crypto = require('node:crypto');
const directory = process.env.CHATGPT_PUBLIC_RUNTIME_DIR;

test('reviewed public source resolves writing delimiters against message metadata', {
  skip: !directory && 'Requires retained public assets; never evaluates downloaded JavaScript.'
}, () => {
  const source = fs.readFileSync(path.join(directory, '4813494d-gf2h57w5fiay19bd.js'), 'utf8');
  assert.equal(crypto.createHash('sha256').update(source).digest('hex'),
    '6c015001732054f4143ef1922609407c540967762109dcd128bbf56706889c3e');
  const index = JSON.parse(fs.readFileSync(path.join(directory, 'shared-symbols.json'), 'utf8')).current;
  const definition = name => index.definitions[name].map(d => source.slice(d.start, d.end)).join('\n');
  assert.equal(definition('Uy'), '`writing`');
  assert.ok(definition('rje').includes('^:::${Uy}'));
  assert.ok(definition('$Ae').includes('t[p]?.content'));
  assert.ok(definition('mje').includes('metadata.writing_blocks'));
  assert.ok(definition('ije').includes('fence.length'));
});

test('normal production part routing reaches the native editor, and export reuses storage', () => {
  const base = path.join(__dirname, '../android/app/src/main/kotlin/com/elon/app');
  const route = fs.readFileSync(path.join(base, 'ChatGptSocialImageContentController.kt'), 'utf8');
  assert.ok(route.indexOf('part.textBlock') < route.indexOf('openOfficialFallback()'));
  assert.match(route, /WebChatTextBlockEditor\(activity, it\)\.show\(\)/);
  const editor = fs.readFileSync(path.join(base, 'WebChatTextBlockEditor.kt'), 'utf8');
  assert.match(editor, /WebChatTextBlockExport\.save/);
  assert.doesNotMatch(editor, /evaluateJavascript|openOfficialFallback|loadUrl/);
  const exporter = fs.readFileSync(path.join(base, 'WebChatTextBlockExport.kt'), 'utf8');
  assert.match(exporter, /ChatGptWebFileByteStorage\.open/);
  assert.match(exporter, /target\.publish\(\)/);
  assert.match(exporter, /target\.discard\(\)/);
});

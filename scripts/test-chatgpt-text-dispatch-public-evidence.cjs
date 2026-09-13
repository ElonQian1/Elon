'use strict';

// Parse pinned public source only. Passing is protocol evidence, not a live send.
const test = require('node:test');
const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');
const crypto = require('node:crypto');
const { parseSource } = require('./analyze-chatgpt-runtime-contracts.cjs');
const directory = process.env.CHATGPT_PUBLIC_RUNTIME_DIR;
const assets = {
  shared: ['4813494d-gf2h57w5fiay19bd.js',
    '6c015001732054f4143ef1922609407c540967762109dcd128bbf56706889c3e'],
  conversation: ['conversation-small-h1dtzoris1y9588z.js',
    'da08c64c132306779e09ba89cac64fa560b120e7560ffdc29b3ce5a0b8ccd67e'],
  composer: ['8b34dbc2-fqgb3eqijpn96umi.js',
    '1d0b132fe9b13120370395bbfbe4bf3324c4dc1213b80e591cd3c6a6db7d16a1']
};

function visit(node, callback) {
  if (!node || typeof node !== 'object') return;
  if (node.type) callback(node);
  for (const value of Object.values(node)) {
    if (Array.isArray(value)) value.forEach(child => visit(child, callback));
    else if (value && typeof value === 'object') visit(value, callback);
  }
}

function definition(module, name, exported = false) {
  const local = exported ? module.exported.get(name) : name;
  assert.ok(local, 'missing exported symbol: ' + name);
  const nodes = module.definitions.get(local);
  assert.equal(nodes?.length, 1, 'ambiguous definition: ' + name);
  return nodes[0];
}

function facts(node) {
  const calls = new Set(), properties = new Set(), strings = new Set();
  visit(node, value => {
    if (value.type === 'CallExpression' && value.callee.type === 'Identifier') calls.add(value.callee.name);
    if (value.type === 'MemberExpression' && !value.computed) properties.add(value.property.name);
    if (value.type === 'Property' && !value.computed) properties.add(value.key.name || value.key.value);
    if (value.type === 'Literal' && typeof value.value === 'string') strings.add(value.value);
    if (value.type === 'TemplateLiteral' && value.expressions.length === 0) {
      strings.add(value.quasis.map(part => part.value.cooked).join(''));
    }
  });
  return { calls, properties, strings };
}

function includes(set, names) {
  for (const name of names) assert.ok(set.has(name), 'missing observed dependency: ' + name);
}

test('pinned text-dispatch boundaries remain distinct from independent HTTP delivery', {
  skip: !directory && 'Set CHATGPT_PUBLIC_RUNTIME_DIR; no website code is imported or executed.'
}, async t => {
  const modules = {};
  for (const [role, [file, hash]] of Object.entries(assets)) {
    const source = fs.readFileSync(path.join(directory, file));
    assert.equal(crypto.createHash('sha256').update(source).digest('hex'), hash, role);
    modules[role] = parseSource(source.toString('utf8'));
  }
  const { conversation, composer, shared } = modules;

  await t.test('fresh request uses the reviewed security-aware client and header builder', () => {
    assert.equal(shared.exported.get('b4'), 'K');
    const client = facts(definition(shared, 'Sd'));
    includes(client.properties, ['disableAutomaticRetry', 'signal', 'requestBody', 'additionalHeaders']);
    const header = facts(definition(shared, 'ac', true));
    includes(header.strings, ['OpenAI-Sentinel-Chat-Requirements-Token',
      'OpenAI-Sentinel-Chat-Requirements-Prepare-Token', 'OpenAI-Sentinel-Turnstile-Token', 'OpenAI-Sentinel-Proof-Token']);
    assert.equal(shared.exported.get('xJ'), 'wS');
    assert.equal(shared.exported.get('Pc'), 'h$');
    assert.equal(conversation.exported.get('FKt'), 'Uun');
    assert.equal(conversation.exported.get('MKt'), 'Xun');
  });

  await t.test('fresh integrity material has an exported website provider', () => {
    const fresh = facts(definition(conversation, 'VKt', true));
    includes(fresh.properties, ['chatReq', 'turnstileToken', 'proofToken', 'force_login',
      'getEnforcementTokenSync', 'getEnforcementToken']);
    assert.deepEqual(composer.imports.get('THe'), { file: './' + assets.conversation[0], name: 'VKt' });
    assert.deepEqual(composer.imports.get('Iee'), { file: './4813494d-gf2h57w5fiay19bd.js', name: 'ac' });
  });

  await t.test('authoritative history can fetch and apply without reloading the document', () => {
    assert.equal(conversation.exported.get('BEn'), 'gy');
    const hydrate = facts(definition(conversation, 'BEn', true));
    includes(hydrate.calls, ['hy', 'fy']);
    includes(hydrate.properties, ['forceNetworkFetch', 'includeMessageId', 'signal',
      'shouldApplyResponse', 'onConversationLoadedFromNetwork', 'skipIfExisting']);
    const apply = facts(definition(conversation, 'KEn', true));
    includes(apply.calls, ['Dbe', 'du']);
    includes(apply.properties, ['existingCurrentLeafId', 'serverCurrentLeafId', 'setCurrentLeafId']);
  });

  await t.test('stream transport also performs request and response integrity checks', () => {
    const stream = facts(definition(conversation, 'jGt', true));
    includes(stream.properties, ['expectedState', 'onBeforeRequestStart', 'headersToRemove',
      'credentials', 'onAccessTokenRecovered', 'onResourceTiming']);
    includes(stream.calls, ['nce', 'kce', 'Afe', 'E_e']);
    includes(stream.strings, ['include', 'text/event-stream', '[DONE]']);
    const retry = facts(definition(conversation, 'MGt', true));
    includes(retry.calls, ['DM']);
    includes(retry.properties, ['onRetry', 'shouldRetry', 'MAX_RETRY_COUNT']);
    assert.deepEqual(composer.imports.get('HOe'), { file: './' + assets.conversation[0], name: 'MGt' });
  });

  await t.test('prepare owns the selected parent and consumable conversation state', () => {
    const prepare = facts(definition(composer, 'vh', true));
    includes(prepare.strings, ['/f/conversation/prepare']);
    includes(prepare.properties, ['parentMessageId', 'lastPrepareParentMessageId',
      'conduitToken', 'prepareRequestBlocked', 'shouldApplyPrepareResponseToSubmit',
      'thinkingEffort', 'serviceTier', 'systemHints']);
    includes(prepare.calls, ['AB']);
  });

  await t.test('request body builder is local, not an importable API', () => {
    assert.equal([...composer.exported.values()].includes('AB'), false);
    const body = facts(definition(composer, 'AB'));
    includes(body.properties, ['conversation_id', 'parent_message_id', 'model',
      'requested_default_model', 'system_hints', 'history_and_training_disabled',
      'thinking_effort', 'supported_encodings', 'messages']);
    includes(body.calls, ['eQt', 'N1e']);
  });

  await t.test('selected Search and Picture tools have distinct prepare, message and dispatch contracts', () => {
    assert.equal(composer.exported.get('$g'), 'sR');
    assert.deepEqual(composer.imports.get('j_'), { file: './' + assets.shared[0], name: 'uG' });
    const hints = definition(shared, 'uG', true);
    assert.match(shared.text.slice(hints.start, hints.end), /Search:`search`/);
    assert.match(shared.text.slice(hints.start, hints.end), /PictureV2:`picture_v2`/);
    const active = definition(composer, 'aR');
    assert.match(composer.text.slice(active.start, active.end), /e\?\[e,\.\.\.r\]:r/);
    const prepare = definition(composer, 'FB');
    assert.match(composer.text.slice(prepare.start, prepare.end), /completionMetadata:\{systemHints:n,conversationMode:T\}/);
    for (const name of ['CJ', 'SJ']) {
      const node = definition(composer, name), source = composer.text.slice(node.start, node.end);
      assert.match(source, /W\.length>0&&\{system_hints:W\}/);
      assert.match(source, /me=ue\?MA\.CONVERSATION_COMPOSER_WEB_ICON:void 0/);
      assert.match(source, /completionMetadata:\{conversationMode:le,systemHints:W,searchSource:me\}/);
      assert.match(source, /systemHints\.filter\(t=>t===j_\.Search\?\(e=!0,!1\):!0\),e&&\(ke\.forceUseSearch=!0\)/);
      assert.match(source, /ke\.enableMessageFollowups=!0/);
    }
    const transform = definition(conversation, 'Lht', true);
    assert.match(conversation.text.slice(transform.start, transform.end), /forceUseSearch:t\.forceUseSearch\?\?\(e\.includes\(Au\.Search\)\?!0:void 0\)/);
    const enumSource = definition(conversation, 'qin', true);
    assert.match(conversation.text.slice(enumSource.start, enumSource.end), /CONVERSATION_COMPOSER_WEB_ICON=`conversation_composer_web_icon`/);
    const dispatch = definition(composer, 'VZt'), body = definition(composer, 'AB');
    assert.match(composer.text.slice(dispatch.start, dispatch.end), /THe\(e\.completionMetadata\)/);
    assert.match(composer.text.slice(body.start, body.end), /force_use_search:MB\(e\.forceUseSearch\)/);
    assert.match(composer.text.slice(body.start, body.end), /client_reported_search_source:e\.completionMetadata\?\.searchSource/);
  });

  await t.test('dispatch consumes prepare ownership and handles stream handoff', () => {
    assert.equal([...composer.exported.values()].includes('VZt'), false);
    const dispatch = facts(definition(composer, 'VZt'));
    includes(dispatch.properties, ['prepareSubmitHandoff', 'onPrepareSubmitHandoffConsumed',
      'onBeforeRequestStart', 'onRequestStart', 'stopConduitToken',
      'onEarlyStreamHandoffControlReceived', 'markAuthoritativeStreamCompletion']);
    includes(dispatch.calls, ['HOe', 'THe', 'IB', 'ZZt', 'iQt', 'rNe', 'QZt']);
    includes(dispatch.strings, ['x-conduit-token', '/f/conversation']);
  });

  await t.test('project sends preserve mode and use account-owned locked-project headers', () => {
    const source = (module, name, exported = false) => {
      const node = definition(module, name, exported); return module.text.slice(node.start, node.end);
    };
    assert.deepEqual(composer.imports.get('wh'), { file: './' + assets.shared[0], name: 'i0' });
    assert.equal(source(shared, 'i0', true), 'e=>e');
    assert.match(source(composer, 'AB'), /conversation_mode:wh\(e\.completionMetadata\?\.conversationMode\)/);
    assert.deepEqual(composer.imports.get('Dh'), { file: './' + assets.shared[0], name: 'iS' });
    assert.match(source(shared, 'iS', true), /if\(`gizmo_id`in e\)return e\.gizmo_id/);
    assert.match(source(composer, 'FB'), /conversationModeOverride\?\?s\?\.mode/);
    includes(facts(definition(composer, 'FB')).properties, ['conversationMode', 'businessAgentContext', 'business_agent_id']);
    assert.deepEqual(composer.imports.get('wi'), { file: './' + assets.shared[0], name: 'JO' });
    includes(facts(definition(shared, 'JO', true)).properties, ['persistedContext', 'turns', 'gizmoId', 'conversationId']);
    for (const name of ['CJ', 'SJ']) assert.match(source(composer, name), /delete ce\.gizmo/);
    for (const [local, exported] of [['kee', 'aK'], ['hre', 'iK'], ['Hf', 'Yr'], ['fh', 'h2']]) {
      assert.deepEqual(composer.imports.get(local), { file: './' + assets.shared[0], name: exported });
    }
    includes(facts(definition(composer, 'OB')).calls, ['kee', 'hre', 'Hf', 'fh', 'Dh']);
    assert.match(source(shared, 'iK', true), /n==null\|\|e==null\|\|t==null\|\|e!==t/);
    assert.match(source(shared, 'iK', true), /return vw\(n\)/);
    assert.match(source(shared, 'mw'), /accountKey===pw\(\)\?e\.pin:null/);
    assert.equal(source(shared, 'yw'), '`x-openai-locked-chats-pin`');
    assert.match(source(shared, 'vw'), /\{\[yw\]:e\}/);
    includes(facts(definition(shared, 'Yr', true)).properties, ['getQueryData']);
    includes(facts(definition(shared, 'sUt')).properties, ['project_id']);
  });

  await t.test('server stop uses this prepared conduit, observed gates and explicit async exclusions', () => {
    const node = definition(conversation, 'xWt', true), stop = facts(node);
    includes(stop.strings, ['/stop_conversation', 'x-conduit-token', 'x-oai-turn-trace-id',
      '3922476776', '877631007', 'pro_mode']);
    includes(stop.properties, ['safePost', 'requestBody', 'additionalHeaders', 'conversation_id',
      'exclude_async_types', 'stopConduitToken', 'chime_version']);
    assert.deepEqual(conversation.imports.get('ts'), { file: './' + assets.shared[0], name: 'b5' });
    assert.match(conversation.text.slice(node.start, node.end), /exclude_async_types:T\?\[`pro_mode`\]:\[\]/);
    assert.match(composer.text, /conduitToken:null,stopConduitToken:t/);
    assert.match(shared.text, /case`finished_partial_completion`/);
    assert.match(shared.text, /e\[e\.STREAMING=3\]=`STREAMING`,e\[e\.UNREAD=4\]=`UNREAD`/);
    assert.match(conversation.text, /value:t\.async_status/);
  });
  await t.test('in-band handoff subscribes the existing provider topic with history catchup', () => {
    assert.equal(shared.exported.get('ej'), 'G9e');
    includes(facts(definition(shared, 'ej', true)).properties, ['getTopic']);
    assert.deepEqual(composer.imports.get('ene'), { file: './' + assets.shared[0], name: 'ej' });
    const handoff = facts(definition(composer, 'ZZt'));
    includes(handoff.strings, ['subscribe_ws_topic', 'resume_sse_endpoint']);
    includes(handoff.calls, ['HZt']);
    const reader = facts(definition(composer, 'HZt'));
    includes(reader.properties, ['subscribe', 'includeAllHistory', 'onMessage', 'unsubscribe',
      'onPotentialMissedMessages', 'stream_item_id', 'parent_stream_item_id', 'encoded_item']);
    includes(reader.strings, ['conversation-turn-stream', 'stream-item', 'done', '[DONE]']);
    const topic = facts(definition(shared, 'net'));
    includes(topic.properties, ['state', 'hasEverSubscribed', 'offset', 'catchups', 'recovered']);
  });
});

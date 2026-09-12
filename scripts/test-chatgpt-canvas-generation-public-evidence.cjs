'use strict';
const test = require('node:test'), assert = require('node:assert/strict');
const fs = require('node:fs'), path = require('node:path'), crypto = require('node:crypto');
const { parseSource } = require('./analyze-chatgpt-runtime-contracts.cjs');
const directory = process.env.CHATGPT_PUBLIC_RUNTIME_DIR;

test('original Canvas generator and identity hook mappings match reviewed public source', {
  skip: !directory && 'Requires reviewed local public assets; no network or module execution.'
}, () => {
  const read = (file, hash) => {
    const bytes = fs.readFileSync(path.join(directory, file));
    assert.equal(crypto.createHash('sha256').update(bytes).digest('hex'), hash);
    return parseSource(bytes.toString('utf8'));
  };
  const canvas = read('d3304073-k8khdx5ezvb9oyu8.js', '4c59db7c2db5afb92e5b037b891fe28a99bc8ab185170c737aad9587a7d5b559');
  const shared = read('4813494d-gf2h57w5fiay19bd.js', '6c015001732054f4143ef1922609407c540967762109dcd128bbf56706889c3e');
  const react = read('2340486e-dyt4epctwx2pn2sj.js', 'bd1f145733f12933c92dd18fbb8e982601c65ef22a41dd2f898fc8f357857261');
  const source = (module, local) => {
    const nodes = module.definitions.get(local); assert.equal(nodes?.length, 1);
    return module.text.slice(nodes[0].start, nodes[0].end);
  };
  assert.equal(canvas.exported.get('a'), 'Hn'); assert.equal(canvas.exported.get('i'), 'Wn');
  assert.deepEqual(canvas.imports.get('Ge'), { file: './8b34dbc2-fqgb3eqijpn96umi.js', name: 'Am' });
  assert.deepEqual(canvas.imports.get('le'), { file: './4813494d-gf2h57w5fiay19bd.js', name: 'tP' });
  assert.deepEqual(canvas.imports.get('ve'), { file: './2340486e-dyt4epctwx2pn2sj.js', name: 'Ln' });
  const body = source(canvas, 'Hn');
  for (const contract of ['sourceRange:s', 'g.slice(s.start,s.end)', 'v=Cn(s,g)', 'pe(Fn(p,m,c,_,v)',
    'exclude_after_next_user_message:!0', 'request_completion.canvas.textdoc.use_create_textdoc_turn.1',
    'textdoc_id:p,textdoc_type:m,version:h,textdoc_content_length:g.length', 'selection_metadata:d',
    'open_in_canvas_view:{type:`canvas_textdoc`,id:p}', 'appendMessages:[y]']) assert.ok(body.includes(contract), contract);
  assert.match(source(canvas, 'An'), /Use the update_textdoc tool to make this edit/);
  assert.equal(shared.exported.get('MP'), 'U1e');
  assert.match(source(shared, 'U1e'), /Object.values\(e\)/);
  assert.doesNotMatch(source(shared, 'U1e'), /R1e|new q1e/);
  assert.equal(react.exported.get('An'), 'Mn'); assert.equal(react.exported.get('In'), 'Un');
  assert.match(source(react, 'Un'), /dn.displayName=`IntlProvider`,Mn=dn/);
});

test('real React root captures official-style callback under a provider and leaves no UI/subscriptions', {
  skip: !process.env.CHATGPT_REACT_TEST_MODULES && 'Requires installed React/ReactDOM/jsdom.'
}, async () => {
  const { createRequire } = require('node:module');
  const load = createRequire(process.env.CHATGPT_REACT_TEST_MODULES + '/package.json');
  const { JSDOM } = load('jsdom'), react = load('react');
  const browser = new JSDOM('<!doctype html><html lang="en"><body></body></html>');
  const oldWindow = global.window, oldDocument = global.document;
  global.window = browser.window; global.document = browser.window.document;
  try {
    const dom = load('react-dom'), renderer = load('react-dom/client'), intl = react.createContext(null);
    let subscriptions = 0, calls = 0;
    const hook = () => react.useSyncExternalStore(() => { subscriptions++; return () => subscriptions--; }, () => false);
    const conversation = { id: 'synthetic', serverId$: () => 'cid' };
    const shared = { canvasConversations: () => [conversation], useCanvasSendBlocked: hook, XM: () => ({}),
      HM: { getCurrentLeafId: () => 'leaf', getNode: () => ({ id: 'leaf' }), getParentPromptNode: () => null } };
    Object.assign(browser.window, { __elonChatGptPrivateTextTransactionsEnabled: true,
      __elonChatGptPrivateCanvasDocumentPolicy: require('../android/app/src/main/assets/chatgpt_web_private_canvas_document_policy.js'),
      __elonChatGptPrivateModelContract: { create: () => ({ withRuntimeIdentity: () => ({ account: 'synthetic' }) }) },
      __elonChatGptPrivateRuntimeBindings: { state: () => ({ profile_id: 'web_20260912' }), load: async role => role === 'shared' ? shared : {
        reactApi: () => react, reactDom: () => dom, reactRoot: () => renderer, intlInit() {},
        intlProvider: props => react.createElement(intl.Provider, { value: props.locale }, props.children)
      } } });
    const service = require('../android/app/src/main/assets/chatgpt_web_private_canvas_generation.js').create(browser.window, {
      loadRuntime: async () => ({ i() {}, a() {
        assert.equal(react.useContext(intl), 'en'); hook();
        return react.useCallback(async () => { calls++; }, []);
      } })
    });
    const prepared = await service.prepare({ id: 'cid' }, { id: 'document', content: 'synthetic', documentVersion: 4, documentType: 'document' },
      { prompt: 'Rewrite synthetic text', start: 0, end: 0 }, () => {});
    await prepared.invoke(() => {});
    assert.equal(calls, 1); assert.equal(subscriptions, 0);
    assert.equal(browser.window.document.body.childElementCount, 0);
  } finally { browser.window.close(); global.window = oldWindow; global.document = oldDocument; }
});

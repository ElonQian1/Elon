'use strict';
const base = require('./chatgpt-fresh-text-context');
const runtime = require('../../android/app/src/main/assets/chatgpt_web_private_text_runtime_submit');
const input = require('../../android/app/src/main/assets/chatgpt_web_private_text_input');

function fixture() {
  const f = base.fixture();
  let draft = 'Synthetic owned draft', clock = 0;
  const edits = [], loads = [];
  f.props.isNewThread = false;
  f.binding.shared.subscribeToSharedProps = () => {};
  const body = { children: [] }, document = { body };
  const root = { return: null, stateNode: { containerInfo: document } };
  root.stateNode.current = root; document.__reactContainer$fixture = root;
  const fiber = { return: root, memoizedProps: { conversation: f.selected, composerController: f.binding.controller },
    dependencies: { firstContext: { memoizedValue: f.binding.shared, next: { memoizedValue: f.files } } } };
  root.child = fiber;
  const view = { dom: { isConnected: false }, isDestroyed: false, state: { doc: { toJSON: () => ({
    type: 'doc', content: [{ type: 'paragraph', content: [{ type: 'text', text: draft }] }]
  }) } } };
  const editor = { Ng: () => f.hints, t_: controller => {
    if (controller !== f.binding.controller) throw Error('wrong owner'); return view;
  }, AS: () => ({ content: draft }), VS: (target, value, options) => {
    if (target !== view || options.scrollIntoView !== false) throw Error('wrong editor');
    edits.push(value); draft = value;
  } };
  Object.assign(f.page, { document, setTimeout, clearTimeout, location: { ...f.page.location, origin: 'https://chatgpt.com' },
    __elonChatGptPrivateTextTransactionsEnabled: true, __elonChatGptFreshTextDispatchEnabled: true,
    __elonChatGptPrivateTransport: { copySameOriginRequestHeaders: () => ({ Authorization: 'Bearer synthetic-fixture' }) } });
  const role = name => ({ shared: f.shared, conversation: f.conversation, composer: editor })[name];
  f.page.__elonChatGptPrivateRuntimeBindings = { state: () => ({ profile_id: 'web_20260912' }),
    observed: () => true, peek: role, load: async name => { loads.push(name); return role(name); } };
  f.page.__elonChatGptPrivateTextRuntimeSubmit = runtime.create(f.page);
  const api = input.create(f.page, { context: f.api, now: () => clock });
  return { ...f, root, fiber, view, editor, edits, loads, api, runtime: f.page.__elonChatGptPrivateTextRuntimeSubmit,
    context: f.api, draft: value => { if (value !== undefined) draft = value; return draft; }, advance: value => { clock += value; } };
}
module.exports = { fixture };

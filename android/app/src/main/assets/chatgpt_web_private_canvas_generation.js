(function (root, factory) {
  'use strict';
  const api = Object.freeze({ version: 1, create: factory });
  if (typeof module === 'object' && module.exports) module.exports = api;
  if (root?.location?.origin === 'https://chatgpt.com') root.__elonChatGptPrivateCanvasGeneration = api;
})(typeof window === 'object' ? window : null, function (page, options) {
  'use strict';
  options ||= {};
  const PROFILE = 'web_20260912';
  const URL = 'https://chatgpt.com/cdn/assets/d3304073-k8khdx5ezvb9oyu8.js';
  const fail = code => { throw Error('canvas_' + code); };
  const policy = page.__elonChatGptPrivateCanvasDocumentPolicy;

  function command(document, input) {
    if (typeof input.prompt !== 'string' || !input.prompt.trim() || input.prompt.length > 4000) fail('prompt_invalid');
    policy.offsets(input.prompt);
    const start = input.start, end = input.end, points = policy.offsets(document.content);
    if (!Number.isSafeInteger(start) || !Number.isSafeInteger(end) || start > end ||
        !points.includes(start) || !points.includes(end)) fail('selection_invalid');
    const selected = start !== end;
    return { content: input.prompt, action: 'edit', userMessageType: 'ask_chatgpt',
      ...(selected ? { sourceRange: { start, end },
        selectionMetadata: { selection_type: 'selection', selection_position_range: { start, end } } } : {}),
      sourceEvent: new page.Event('click') };
  }

  async function prepare(binding, document, input, check) {
    const value = command(document, input), bindings = page.__elonChatGptPrivateRuntimeBindings;
    if (page.__elonChatGptPrivateTextTransactionsEnabled !== true || bindings?.state?.().profile_id !== PROFILE) {
      fail('generation_unavailable');
    }
    if (page.__elonChatGptPrivateTextRuntimeSubmit?.state?.().pending ||
        page.__elonChatGptPrivateRegenerateRuntime?.state?.().pending) fail('conversation_busy');
    let timer;
    const load = options.loadRuntime || (url => import(url));
    const [shared, runtime, generator] = await Promise.all([
      bindings.load('shared'), bindings.load('react'),
      Promise.race([Promise.resolve().then(() => load(URL)), new Promise((_, reject) => {
        timer = page.setTimeout(() => reject(Error('canvas_runtime_timeout')), options.timeoutMs || 1500);
      })]).finally(() => page.clearTimeout(timer))
    ]);
    const identity = page.__elonChatGptPrivateModelContract?.create(page);
    const readIdentity = () => identity?.withRuntimeIdentity({}, shared)?.account || null;
    const account = readIdentity();
    if (!account || typeof shared?.canvasConversations !== 'function' || typeof shared.useCanvasSendBlocked !== 'function' ||
        typeof generator?.i !== 'function' || typeof runtime?.intlInit !== 'function' ||
        !['getNode', 'getCurrentLeafId', 'getParentPromptNode'].every(key => typeof shared.HM?.[key] === 'function')) {
      fail('generation_unavailable');
    }
    const conversations = shared.canvasConversations();
    if (!Array.isArray(conversations)) fail('generation_unavailable');
    const matches = conversations.filter(row => typeof row?.serverId$ === 'function' && row.serverId$() === binding.id);
    if (matches.length !== 1) fail('generation_owner_unavailable');
    const conversation = matches[0], tree = () => shared.XM(conversation.id);
    const leaf = shared.HM.getCurrentLeafId(tree());
    if (typeof leaf !== 'string' || !leaf || !shared.HM.getNode(tree(), leaf)) fail('generation_owner_unavailable');
    function current() {
      check();
      if (bindings.state().profile_id !== PROFILE || readIdentity() !== account || conversation.serverId$() !== binding.id ||
          !shared.canvasConversations().includes(conversation)) fail('context_changed');
    }
    current();
    generator.i(); runtime.intlInit();
    const react = runtime.reactApi(), dom = runtime.reactDom(), renderer = runtime.reactRoot();
    if (typeof generator.a !== 'function' || !runtime.intlProvider || typeof react.useLayoutEffect !== 'function' ||
        typeof dom.flushSync !== 'function' || typeof renderer.createRoot !== 'function') fail('generation_unavailable');
    function captureCallback() {
      let callback, blocked, root, failed = false;
      function Snapshot() {
        // The official hook builds hidden Canvas context and preserves its normal request proofs/state.
        const send = generator.a(conversation, { textdocId: document.id, type: document.documentType,
          versionInt: document.documentVersion, content: document.content });
        const denied = shared.useCanvasSendBlocked();
        react.useLayoutEffect(() => { callback = send; blocked = denied; });
        return null;
      }
      try {
        root = renderer.createRoot(page.document.createElement('div'), {
          onUncaughtError: () => { failed = true; }, onCaughtError: () => { failed = true; },
          onRecoverableError: () => { failed = true; }
        });
        const locale = page.document.documentElement?.lang || page.navigator?.language || 'en';
        dom.flushSync(() => root.render(react.createElement(runtime.intlProvider, { locale, messages: {},
          onError: () => {} }, react.createElement(Snapshot))));
      } catch (_) { failed = true; }
      finally { try { if (root) dom.flushSync(() => root.unmount()); } catch (_) { failed = true; } }
      if (failed || typeof callback !== 'function' || typeof blocked !== 'boolean') fail('generation_unavailable');
      if (blocked) fail('generation_blocked');
      return callback;
    }
    current();
    if (shared.HM.getCurrentLeafId(tree()) !== leaf) fail('context_changed');
    let dispatched = false, promptId, completed = false;
    return Object.freeze({
      invoke(beforeDispatch) {
        if (dispatched) fail('write_unconfirmed');
        current();
        if (shared.HM.getCurrentLeafId(tree()) !== leaf) fail('context_changed');
        const callback = captureCallback();
        current();
        if (shared.HM.getCurrentLeafId(tree()) !== leaf) fail('context_changed');
        beforeDispatch();
        dispatched = true;
        // Hn does not await/return requestCompletion. This is dispatch, not server acceptance.
        return callback(value);
      },
      settled() {
        if (!dispatched) return false;
        current();
        if (completed) return true;
        const state = tree(), node = shared.HM.getNode(state, shared.HM.getCurrentLeafId(state));
        const message = node?.message;
        const user = message?.author?.role === 'user' ? node : shared.HM.getParentPromptNode(state, node?.id);
        const metadata = user?.message?.metadata?.canvas;
        const text = user?.message?.content;
        const selection = metadata?.selection_metadata, range = selection?.selection_position_range;
        const sameSelection = value.sourceRange ? selection?.selection_type === 'selection' &&
          range?.start === value.sourceRange.start && range?.end === value.sourceRange.end : selection == null;
        if (!user?.id || user.id === leaf || promptId && promptId !== user.id ||
            user.message?.author?.role !== 'user' || text?.content_type !== 'text' ||
            !Array.isArray(text.parts) || text.parts.length !== 1 || text.parts[0] !== input.prompt ||
            metadata?.textdoc_id !== document.id || metadata.textdoc_type !== document.documentType ||
            metadata.version !== document.documentVersion || metadata.textdoc_content_length !== document.content.length ||
            metadata.user_message_type !== 'ask_chatgpt' || !sameSelection) return false;
        promptId = user.id;
        completed = message?.author?.role === 'assistant' &&
          ['finished_successfully', 'finished_partial_completion'].includes(message.status);
        return completed;
      }
    });
  }
  return Object.freeze({ prepare });
});

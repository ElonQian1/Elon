(function (root, factory) {
  'use strict';
  const api = Object.freeze({ version: 1, create: factory });
  if (typeof module === 'object' && module.exports) module.exports = api;
  if (root?.location?.origin === 'https://chatgpt.com' &&
      !(Number(root.__elonChatGptRuntimeGenerationState?.version) >= api.version)) {
    root.__elonChatGptRuntimeGenerationState = factory(root);
  }
})(typeof window === 'object' ? window : null, function (page) {
  'use strict';
  const unknown = code => ({ active: null, code });
  const validId = value => value == null ||
    typeof value === 'string' && /^[a-z0-9_-]{1,180}$/i.test(value);
  const id = value => value == null ? null : value;

  function pendingWriter() {
    return page.__elonChatGptPrivateTextRuntimeSubmit?.state?.().pending === true ||
      page.__elonChatGptPrivateTextTransactionRelay?.state?.().active === true ||
      page.__elonChatGptPrivateRegenerateRuntime?.state?.().pending === true ||
      page.__elonChatGptPrivateStopRuntime?.state?.().pending === true;
  }

  function known(shared) {
    return typeof shared?.XM === 'function' && typeof shared.HM?.getRequestId === 'function' &&
      typeof shared.Fl === 'function' && typeof shared.Fx === 'function' &&
      shared.v7 && ['STREAMING', 'UNREAD', 'REALTIME', 'REALTIME_BUSY', 'REALTIME_BACKGROUND']
        .every((key, index) => shared.v7[key] === index + 3);
  }

  function read(composer, stream) {
    try {
      if (page.__elonChatGptPrivateTextTransactionsEnabled !== true) return unknown('disabled');
      if (stream?.state !== 'completed' || !stream.id || !validId(stream.id)) return unknown('stream_unconfirmed');
      if (pendingWriter()) return unknown('writer_pending');
      const runtime = page.__elonChatGptPrivateTextRuntimeSubmit;
      const binding = runtime?.captureConversation?.(composer, true);
      const shared = page.__elonChatGptPrivateRuntimeBindings?.peek?.('shared');
      if (!binding || !known(shared)) return unknown('runtime_unavailable');
      const props = binding.shared.getSharedProps();
      const conversation = binding.conversation, requestId = binding.requestId;
      // The stream must belong to the mounted branch, not a prior reply whose
      // completion happened to be cached for the same conversation.
      if (props.conversation !== conversation || props.composerController !== binding.controller ||
          props.currentLeafId !== stream.id || !validId(requestId) ||
          id(props.currentRequestId) !== id(requestId) ||
          stream.conversationId && binding.conversationId &&
            stream.conversationId !== binding.conversationId) return unknown('turn_mismatch');
      const tree = shared.XM(conversation.id);
      if (!tree || id(shared.HM.getRequestId(tree)) !== id(requestId)) return unknown('request_mismatch');
      const active = shared.Fl(requestId), status = shared.Fx(conversation);
      const after = binding.shared.getSharedProps();
      if (!composer.isConnected || page.__elonChatGptDocumentToken !== binding.token ||
          page.location.href !== binding.href || after.conversation !== conversation ||
          after.composerController !== binding.controller || after.currentLeafId !== stream.id ||
          id(after.currentRequestId) !== id(requestId) || pendingWriter() ||
          id(shared.HM.getRequestId(tree)) !== id(requestId)) return unknown('context_changed');
      if (typeof active !== 'boolean') return unknown('request_state_unknown');
      const mode = status == null ? null : status.value;
      if (active || [shared.v7.STREAMING, shared.v7.REALTIME, shared.v7.REALTIME_BUSY,
        shared.v7.REALTIME_BACKGROUND].includes(mode)) return { active: true, code: 'request_active' };
      if (mode !== null && mode !== shared.v7.UNREAD) return unknown('generation_state_unknown');
      return { active: false, code: 'completed_current_turn' };
    } catch (_) { return unknown('runtime_unavailable'); }
  }

  // Reuse the versioned shared-module cache once. Reads never import modules,
  // issue HTTP requests, alter the website state or create polling timers.
  try {
    const bindings = page.__elonChatGptPrivateRuntimeBindings;
    if (page.__elonChatGptPrivateTextTransactionsEnabled === true &&
        bindings?.observed?.('shared') && !bindings.peek('shared')) bindings.load('shared').catch(() => {});
  } catch (_) {}
  return Object.freeze({ version: 1, read });
});

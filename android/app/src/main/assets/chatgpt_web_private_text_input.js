(function (root, factory) {
  'use strict';
  const api = Object.freeze({ version: 6, create: factory });
  if (typeof module === 'object' && module.exports) module.exports = api;
  if (root?.location?.origin === 'https://chatgpt.com') root.__elonChatGptPrivateTextInput = factory(root);
})(typeof window === 'object' ? window : null, function (page, options) {
  'use strict';
  options ||= {};
  const context = options.context || page.__elonChatGptFreshTextContext.create(page);
  const now = options.now || Date.now;
  let cached = null, pending = null, retryAfter = 0, lastStamp = null, lastDocument = page.document;

  function scope() {
    return { allowNewConversations: page.__elonChatGptFreshTextNewConversationsEnabled !== false,
      allowProjects: page.__elonChatGptFreshTextProjectsEnabled === true,
      allowTemporary: page.__elonChatGptFreshTextTemporaryEnabled === true,
      allowTools: page.__elonChatGptFreshTextToolsEnabled === true,
      allowPersonalSearch: page.__elonChatGptFreshTextToolsEnabled !== false,
      allowPersonalImage: page.__elonChatGptFreshTextToolsEnabled !== false,
      allowAttachments: page.__elonChatGptFreshTextAttachmentsEnabled === true,
      allowPersonalAttachments: page.__elonChatGptFreshTextPersonalAttachmentsEnabled !== false,
      requireNativeAttachment: false };
  }

  function stamp() {
    const identity = context.stamp();
    return identity ? JSON.stringify([identity, scope()]) : null;
  }

  function enabled() {
    return page.__elonChatGptPrivateTextTransactionsEnabled === true && page.__elonChatGptFreshTextDispatchEnabled !== false;
  }

  function snapshot(composer, notify) {
    const empty = { ready: false, draft: null };
    try {
      const currentStamp = enabled() ? stamp() : null;
      if (currentStamp !== lastStamp || page.document !== lastDocument) {
        cached = null; retryAfter = 0; lastStamp = currentStamp; lastDocument = page.document;
        // Retire this observer only; shared runtime loads may have other consumers.
        if (pending) page.clearTimeout(pending.timer);
        pending = null;
      }
      if (!currentStamp || composer) return empty;
      if (cached?.binding.current()) {
        const draft = cached.binding.draft?.read();
        if (typeof draft === 'string') return { ready: true, draft };
      }
      // Recheck accepted personal tools promptly; failed captures still back off.
      const previous = cached?.binding;
      if (previous && !previous.newConversation && !previous.temporary && !previous.projectId &&
          !previous.attachments && ['search', 'picture_v2'].includes(previous.tool)) retryAfter = 0;
      cached = null;
      // Capture validates actual account, parent, model, files, tools and the
      // official in-memory draft. No preparation request or submit is issued.
      if (!pending && now() >= retryAfter) {
        const attempt = { document: page.document, stamp: currentStamp, timer: null };
        pending = attempt;
        attempt.timer = page.setTimeout(() => {
          if (pending !== attempt) return;
          pending = null; retryAfter = now() + 2000;
        }, 5000);
        void context.capture(null, null, scope()).then(binding => {
          if (pending === attempt && enabled() && page.document === attempt.document && stamp() === attempt.stamp &&
              binding.current() && typeof binding.draft?.read() === 'string') cached = { binding };
        }).catch(() => {}).finally(() => {
          if (pending !== attempt) return;
          page.clearTimeout(attempt.timer);
          pending = null;
          retryAfter = now() + 2000;
          if (cached && enabled() && page.document === attempt.document && stamp() === attempt.stamp) {
            try { notify?.(); } catch (_) {}
          }
        });
      }
      return empty;
    } catch (_) { cached = null; retryAfter = now() + 2000; return empty; }
  }

  function setDraft(value, expected) {
    try {
      if (!enabled() || stamp() !== lastStamp || !cached?.binding.current()) return false;
      return cached.binding.draft.replace(value, expected) === true;
    } catch (_) { return false; }
  }

  function setCommand(value, expected, respond, io) {
    const composer = io.find();
    if (!composer && setDraft(value, expected)) {
      respond('set_draft', true, ''); return io.notify(true);
    }
    if (!composer) return respond('set_draft', false, '未找到输入框，请切换网页模式。');
    if (io.compare(io.read(composer)) !== io.compare(expected)) {
      return respond('set_draft', false, '网页草稿已变化，请返回官网确认后重试。');
    }
    if (!io.write(composer, value)) return respond('set_draft', false, '官方输入框未接受文本，请返回官网重试。');
    respond('set_draft', true, ''); io.notify();
  }

  return Object.freeze({ version: 6, snapshot, setDraft, setCommand });
});

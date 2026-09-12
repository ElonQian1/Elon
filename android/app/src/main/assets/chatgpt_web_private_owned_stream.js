(function (root, factory) {
  'use strict';
  const api = Object.freeze({ version: 1, create: factory });
  if (typeof module === 'object' && module.exports) module.exports = api;
  if (root?.location?.origin === 'https://chatgpt.com') root.__elonChatGptPrivateOwnedStream = api;
})(typeof window === 'object' ? window : null, function (options) {
  'use strict';
  let owner = null;
  const active = () => !!owner && owner.current();
  function begin({ conversationId, current }) {
    if (!conversationId || typeof current !== 'function' || !current()) return null;
    const next = { current, failure: '', done: false };
    owner = next;
    options.session.begin();
    const owns = () => owner === next && current();
    const decoder = options.policy.createSseDecoder(payload => {
      if (!owns()) { next.failure = 'context_changed'; return; }
      const id = options.conversationId(payload);
      if (id && id !== conversationId) { next.failure = 'stream_owner_changed'; return; }
      options.report?.(payload);
      if (options.session.accept(payload)) { options.rich?.(payload); options.notify(); }
    }, ending => {
      if (!owns()) return;
      if (ending?.error || ending?.interrupted) { next.failure = 'stream_decode_failed'; return; }
      next.done = true;
      if (options.session.finish()) options.notify();
    }, { strict: true, requireDone: true });
    function check() {
      if (!owns()) throw Error('context_changed');
      if (next.failure) throw Error(next.failure);
    }
    return Object.freeze({
      push(value) {
        check();
        if (next.done || !value || !('data' in value)) return;
        const event = value.event || '';
        if (typeof event !== 'string' || /[\r\n]/.test(event)) throw Error('stream_item_invalid');
        const data = JSON.stringify(value.data);
        if (!data || data.length > 1024 * 1024) throw Error('stream_item_invalid');
        decoder.push((event ? 'event: ' + event + '\n' : '') + 'data: ' + data + '\n\n');
        check();
        if (!decoder.resumable() && !next.done) throw Error('stream_decode_failed');
      },
      finish() {
        check();
        decoder.push('data: [DONE]\n\n');
        check();
      }
    });
  }
  return Object.freeze({ begin, active, reset() { owner = null; } });
});

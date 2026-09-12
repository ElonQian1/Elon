(function (root, factory) {
  'use strict';
  const api = Object.freeze({ version: 1, create: factory });
  if (typeof module === 'object' && module.exports) module.exports = api;
  if (root?.location?.origin === 'https://chatgpt.com') root.__elonChatGptFreshTextStream = api;
})(typeof window === 'object' ? window : null, function (options) {
  'use strict';
  const { signal, current, getTopic } = options;
  const setTimer = options.setTimeout || setTimeout;
  const clearTimer = options.clearTimeout || clearTimeout;
  const idleMs = options.idleTimeoutMs || 45000;
  function check() {
    if (signal.aborted) throw Error('cancelled');
    if (!current()) throw Error('context_changed');
  }
  function decode(encoded) {
    if (typeof encoded !== 'string' || encoded.length > 1024 * 1024) throw Error('stream_item_invalid');
    let event, data = [];
    for (const line of encoded.replace(/\r\n?/g, '\n').split('\n')) {
      if (line.startsWith('event:')) event = line.slice(6).trim();
      if (line.startsWith('data:')) data.push(line.slice(5).trimStart());
    }
    const text = data.join('\n').trim();
    if (!text || text === '[DONE]' || event === 'ping') return null;
    let value;
    try { value = JSON.parse(text); }
    catch (_) {
      if (event === 'delta_encoding' && text === 'v1') value = text;
      else throw Error('stream_item_invalid');
    }
    if (value?.error) throw Error('stream_server_error');
    return { event, data: value };
  }
  async function* topicStream(topicId) {
    check();
    if (typeof getTopic !== 'function' || typeof topicId !== 'string' ||
        !topicId || topicId.length > 512 || /[\s\x00-\x1f]/.test(topicId)) throw Error('stream_handoff_unavailable');
    const topic = getTopic(topicId);
    // The provider's topics are cached, not reference counted. Never take over
    // a topic already used by the website or unsubscribe another consumer.
    if (!topic || topic.state !== 2 || topic.hasEverSubscribed === true ||
        typeof topic.onMessage !== 'function' || typeof topic.onPotentialMissedMessages !== 'function' ||
        typeof topic.subscribe !== 'function' || typeof topic.unsubscribe !== 'function') {
      throw Error('stream_topic_owned');
    }
    const queue = [], seen = new Set(), off = [];
    let bytes = 0, timer, wake, failure, done = false, closed = false, subscribed = false;
    const notify = () => { const next = wake; wake = null; next?.(); };
    const fail = code => { if (!closed) { failure ||= Error(code); done = true; notify(); } };
    const refresh = () => { clearTimer(timer); timer = setTimer(() => fail('stream_topic_timeout'), idleMs); };
    const abort = () => fail('cancelled');
    function message(event) {
      if (closed || done || event?.type !== 'conversation-turn-stream') return;
      try {
        check(); refresh();
        const payload = event.payload;
        if (payload?.type === 'done') { done = true; clearTimer(timer); notify(); return; }
        if (payload?.type !== 'stream-item') return;
        const { stream_item_id: id, parent_stream_item_id: parent, encoded_item: encoded } = payload;
        if (typeof id !== 'string' || !id || id.length > 512) throw Error('stream_item_invalid');
        if (seen.has(id)) return;
        if (parent && !seen.has(parent)) throw Error('stream_item_gap');
        if (seen.size >= 32768) throw Error('stream_item_limit');
        const value = decode(encoded);
        seen.add(id);
        if (value) {
          if (queue.length >= 256 || bytes + encoded.length > 4 * 1024 * 1024) throw Error('stream_queue_limit');
          queue.push({ value, size: encoded.length }); bytes += encoded.length;
        }
        notify();
      } catch (error) { fail(error.message); }
    }
    try {
      off.push(topic.onMessage(message), topic.onPotentialMissedMessages(() => fail('stream_item_gap')));
      signal.addEventListener('abort', abort, { once: true });
      check(); refresh(); subscribed = true;
      // Register before subscribe: history catchups may be delivered immediately.
      Promise.resolve(topic.subscribe({ includeAllHistory: true })).catch(() => fail('stream_subscribe_failed'));
      for (;;) {
        check();
        if (failure) throw failure;
        if (queue.length) { const item = queue.shift(); bytes -= item.size; yield item.value; }
        else if (done) return;
        else await new Promise(resolve => { wake = resolve; });
      }
    } finally {
      closed = true; clearTimer(timer); signal.removeEventListener('abort', abort);
      for (const dispose of off) { try { dispose?.(); } catch (_) {} }
      if (subscribed) { try { Promise.resolve(topic.unsubscribe()).catch(() => {}); } catch (_) {} }
      queue.length = 0; seen.clear(); notify();
    }
  }
  async function* follow(source) {
    try {
      for await (const value of source) {
        check();
        yield value;
        if (value?.data?.type !== 'stream_handoff') continue;
        const choices = value.data.options;
        const option = Array.isArray(choices) && choices.find(item => item?.type === 'subscribe_ws_topic');
        if (!option) throw Error('stream_handoff_unavailable');
        yield* topicStream(option.topic_id);
        // Root SSE completion is not completion of the handed-off turn.
        return;
      }
    } finally {
      try { Promise.resolve(source.return?.()).catch(() => {}); } catch (_) {}
    }
  }
  return Object.freeze({ follow });
});

(function () {
  'use strict';

  const root = window;
  if (root.__elonWinChatGptRealtimeVoiceTranscript || location.origin !== 'https://chatgpt.com') return;
  const deltaFactory = root.__elonWinChatGptRealtimeVoiceJsonDelta;
  const messageAdapter = root.__elonChatGptMessages;
  if (!deltaFactory || typeof deltaFactory.create !== 'function' || !messageAdapter) return;

  const MAX_PAYLOAD_CHARS = 256 * 1024;
  const MAX_NESTED_JSON_CHARS = 128 * 1024;
  const MAX_TRANSCRIPT_CHARS = 64 * 1024;
  const MAX_STREAMS = 32;
  const MAX_CONTENT_PARTS = 32;
  const MAX_EVENT_HASHES = 256;
  const MAX_ROUTE_CHARS = 512;
  const TERMINAL_STATES = new Set(['completed', 'finished', 'finished_successfully', 'done']);
  const ASSISTANT_DELTA_TYPES = new Set([
    'response.output_audio_transcript.delta', 'response.audio_transcript.delta',
  ]);
  const ASSISTANT_FINAL_TYPES = new Set([
    'response.output_audio_transcript.done', 'response.audio_transcript.done',
  ]);
  const USER_DELTA_TYPES = new Set(['conversation.item.input_audio_transcription.delta']);
  const USER_FINAL_TYPES = new Set(['conversation.item.input_audio_transcription.completed']);
  const DIRECT_TEXT_FIELDS = ['text', 'transcript', 'caption'];
  const ENVELOPE_KEYS = ['event', 'data', 'message', 'payload', 'body'];
  const boundPeers = new WeakSet();
  const boundChannels = new WeakSet();
  const openChannels = new Set();
  const connectedChannels = new Set();
  const streams = new Map();
  const seenHashes = new Map();
  let deltaDecoder = deltaFactory.create();
  let previousTextByStream = new Map();
  let route = '';
  let nextOrder = 0;
  let revision = 0;
  let observedFrameCount = 0;
  let acceptedEventCount = 0;
  let snapshotTimer = 0;
  let stateTimer = 0;

  function structuralState() {
    return Object.freeze({
      version: 1,
      active: connectedChannels.size > 0,
      observedChannelCount: openChannels.size,
      openChannelCount: connectedChannels.size,
      observedFrameCount,
      acceptedEventCount,
      streamCount: streams.size,
      revision,
    });
  }

  function emitStructuralState() {
    stateTimer = 0;
    const nativeBridge = root.elonChatGptNative;
    const documentToken = String(root.__elonChatGptDocumentToken || '');
    const adapterVersion = Number(root.__elonChatGptAdapterVersion || 0);
    if (!nativeBridge || typeof nativeBridge.postMessage !== 'function' ||
      !validIdentifier(documentToken) || !Number.isInteger(adapterVersion) || adapterVersion < 1) return;
    try {
      nativeBridge.postMessage(JSON.stringify({
        schema: 'yilong.ai.ui.v1',
        adapterVersion,
        documentToken,
        providerId: 'chatgpt',
        source: 'official_web',
        conversationId: cleanRoute(location.pathname),
        emittedAt: new Date().toISOString(),
        event: { type: 'realtime_voice_state', ...structuralState() },
      }));
    } catch (_) {}
  }

  function scheduleStructuralState() {
    if (stateTimer) return;
    stateTimer = window.setTimeout(emitStructuralState, 40);
  }

  function cleanRoute(value) {
    try {
      const path = String(value || location.pathname || '/').slice(0, MAX_ROUTE_CHARS);
      return path.startsWith('/') ? path : '/';
    } catch (_) { return '/'; }
  }

  function validToken(value) {
    return typeof value === 'string' && value.length >= 1 && value.length <= 96 &&
      /^[A-Za-z0-9._-]+$/.test(value);
  }

  function validIdentifier(value) {
    return typeof value === 'string' && value.length >= 1 && value.length <= 192 &&
      /^[A-Za-z0-9_:.\-]+$/.test(value);
  }

  function parseJson(value, limit) {
    if (typeof value !== 'string' || value.length < 2 || value.length > limit) return null;
    try {
      const parsed = JSON.parse(value);
      return parsed && typeof parsed === 'object' && !Array.isArray(parsed) ? parsed : null;
    } catch (_) { return null; }
  }

  function nestedObject(value) {
    if (value && typeof value === 'object' && !Array.isArray(value)) return value;
    if (Array.isArray(value)) {
      for (let index = 0; index < Math.min(value.length, MAX_CONTENT_PARTS); index += 1) {
        const nested = nestedObject(value[index]);
        if (nested) return nested;
      }
      return null;
    }
    return parseJson(value, MAX_NESTED_JSON_CHARS);
  }

  function eventObject(payload) {
    let current = parseJson(payload, MAX_PAYLOAD_CHARS);
    if (!current) return null;
    for (let depth = 0; depth < 3; depth += 1) {
      const type = typeof current.type === 'string' ? current.type : '';
      if (type && type !== 'data_message') return current;
      let nested = null;
      for (const key of ENVELOPE_KEYS) {
        nested = nestedObject(current[key]);
        if (nested) break;
      }
      if (!nested) return current;
      current = nested;
    }
    return current;
  }

  function descriptor(type) {
    if (ASSISTANT_DELTA_TYPES.has(type)) return { role: 'assistant', update: 'delta' };
    if (ASSISTANT_FINAL_TYPES.has(type)) return { role: 'assistant', update: 'final' };
    if (USER_DELTA_TYPES.has(type)) return { role: 'user', update: 'delta' };
    if (USER_FINAL_TYPES.has(type)) return { role: 'user', update: 'final' };
    return null;
  }

  function directEvent(event) {
    const type = validToken(event.type) ? event.type : '';
    const info = descriptor(type);
    if (!info) return null;
    const text = event[info.update === 'delta' ? 'delta' : 'transcript'];
    const streamKey = [event.item_id, event.response_id].find(validIdentifier);
    if (typeof text !== 'string' || !text || text.length > MAX_TRANSCRIPT_CHARS || !streamKey) return null;
    return {
      eventId: validIdentifier(event.event_id) ? event.event_id : '',
      streamKey, role: info.role, update: info.update, text,
    };
  }

  function eventId(event) {
    const nested = event.payload && typeof event.payload === 'object' ? event.payload.event_id : '';
    return [event.event_id, nested].find(validIdentifier) || '';
  }

  function partText(value) {
    if (typeof value === 'string') return value.length <= MAX_TRANSCRIPT_CHARS ? value : '';
    if (!value || typeof value !== 'object' || Array.isArray(value)) return '';
    for (const field of DIRECT_TEXT_FIELDS) {
      if (typeof value[field] === 'string' && value[field].length <= MAX_TRANSCRIPT_CHARS) {
        return value[field];
      }
    }
    return '';
  }

  function messageText(content) {
    if (!content || typeof content !== 'object') return '';
    if (Array.isArray(content.parts)) {
      return content.parts.slice(0, MAX_CONTENT_PARTS).map(partText).filter(Boolean)
        .join('\n\n').slice(0, MAX_TRANSCRIPT_CHARS);
    }
    return DIRECT_TEXT_FIELDS.map((field) => content[field]).find((value) =>
      typeof value === 'string'
    )?.slice(0, MAX_TRANSCRIPT_CHARS) || '';
  }

  function messageRole(message, content) {
    const role = message && message.author && message.author.role;
    if (role === 'user' || role === 'assistant') return role;
    if (!Array.isArray(content && content.parts)) return '';
    const audio = content.parts.slice(0, MAX_CONTENT_PARTS).find((part) =>
      part && typeof part === 'object' && part.content_type === 'audio_transcription'
    );
    return audio && audio.direction === 'in' ? 'user'
      : audio && audio.direction === 'out' ? 'assistant' : '';
  }

  function trimPreviousStreams() {
    while (previousTextByStream.size > MAX_STREAMS) {
      previousTextByStream.delete(previousTextByStream.keys().next().value);
    }
  }

  function messageEvent(event, message, defaultFinal) {
    if (!message || typeof message !== 'object') return null;
    const streamKey = [message.id, message.message_id].find(validIdentifier);
    const content = message.content;
    const role = messageRole(message, content);
    const currentText = messageText(content);
    if (!streamKey || !role || !currentText.trim()) return null;
    const terminal = defaultFinal === true || message.end_turn === true ||
      TERMINAL_STATES.has(String(message.status || '').toLowerCase());
    const previous = previousTextByStream.get(streamKey) || '';
    const compatibleDelta = !previous || currentText.startsWith(previous);
    const update = terminal || !compatibleDelta ? 'final' : 'delta';
    const text = update === 'final' ? currentText : currentText.slice(previous.length);
    previousTextByStream.set(streamKey, currentText);
    trimPreviousStreams();
    if (!text && update === 'delta') return null;
    return { eventId: eventId(event), streamKey, role, update, text };
  }

  function decodeDirectPrivateText(event, role) {
    const body = event.payload && typeof event.payload === 'object' ? event.payload : null;
    if (!body) return null;
    const streamKey = [body.message_id, body.item_id, body.turn_id].find(validIdentifier);
    const text = DIRECT_TEXT_FIELDS.map((field) => body[field]).find((value) =>
      typeof value === 'string' && value.trim() && value.length <= MAX_TRANSCRIPT_CHARS
    );
    if (!streamKey || !text) return null;
    const terminal = body.final === true || TERMINAL_STATES.has(String(body.status || '').toLowerCase());
    return messageEvent(event, {
      id: streamKey,
      author: { role },
      content: { parts: [text] },
      end_turn: terminal,
    }, terminal);
  }

  function decodePayload(payload) {
    const event = eventObject(payload);
    if (!event) return null;
    const direct = directEvent(event);
    if (direct) return direct;
    if (event.type === 'chat_message_delta') {
      const delta = event.payload && event.payload.delta;
      const decoded = delta && deltaDecoder.apply(delta);
      return decoded && decoded.message ? messageEvent(event, decoded.message, false) : null;
    }
    if (event.type === 'full_chat_message') {
      return messageEvent(event, event.payload && event.payload.message, true);
    }
    if (event.type === 'user_transcription_text') return decodeDirectPrivateText(event, 'user');
    if (event.type === 'live_captioning_text') return decodeDirectPrivateText(event, 'assistant');
    return null;
  }

  function payloadHash(value) {
    let hash = 2166136261;
    for (let index = 0; index < value.length; index += 1) {
      hash ^= value.charCodeAt(index);
      hash = Math.imul(hash, 16777619);
    }
    return (hash >>> 0).toString(16) + ':' + value.length;
  }

  function rememberHash(hash) {
    if (seenHashes.has(hash)) return false;
    seenHashes.set(hash, true);
    while (seenHashes.size > MAX_EVENT_HASHES) seenHashes.delete(seenHashes.keys().next().value);
    return true;
  }

  function requestSnapshot() {
    if (snapshotTimer) return;
    snapshotTimer = window.setTimeout(() => {
      snapshotTimer = 0;
      const bridge = root.__elonChatGptBridge;
      const documentToken = String(root.__elonChatGptDocumentToken || '');
      if (!bridge || typeof bridge.command !== 'function' || !validIdentifier(documentToken)) return;
      try { bridge.command(JSON.stringify({ action: 'snapshot', documentToken })); } catch (_) {}
    }, 60);
  }

  function trimStreams() {
    while (streams.size > MAX_STREAMS) streams.delete(streams.keys().next().value);
  }

  function applyTranscript(event) {
    const existing = streams.get(event.streamKey);
    const previous = existing && existing.text || '';
    const text = event.update === 'final' ? event.text : previous + event.text;
    if (!text.trim() || text.length > MAX_TRANSCRIPT_CHARS) return false;
    const unchanged = existing && existing.text === text &&
      existing.state === (event.update === 'final' ? 'completed' : 'streaming');
    if (unchanged) return false;
    streams.set(event.streamKey, {
      id: 'voice-' + event.streamKey,
      role: event.role,
      state: event.update === 'final' ? 'completed' : 'streaming',
      text,
      order: existing ? existing.order : nextOrder++,
    });
    trimStreams();
    acceptedEventCount += 1;
    revision += 1;
    requestSnapshot();
    scheduleStructuralState();
    return true;
  }

  function acceptPayload(payload) {
    if (typeof payload !== 'string' || !payload || payload.length > MAX_PAYLOAD_CHARS) return false;
    observedFrameCount += 1;
    const event = decodePayload(payload);
    if (!event) return false;
    const hash = event.eventId ? 'id:' + event.eventId : 'body:' + payloadHash(payload);
    return rememberHash(hash) && applyTranscript(event);
  }

  function comparableText(message) {
    return Array.isArray(message && message.content) ? message.content.map((part) =>
      part && typeof part.text === 'string' ? part.text : ''
    ).filter(Boolean).join('\n\n').replace(/\s+/g, ' ').trim() : '';
  }

  function authoritativeMatch(messages, stream) {
    const expected = stream.text.replace(/\s+/g, ' ').trim();
    return messages.some((message) => {
      if (!message || message.role !== stream.role) return false;
      if (message.id === stream.id || message.id === stream.id.slice(6)) return true;
      const actual = comparableText(message);
      return actual === expected || (stream.state === 'streaming' && actual && expected.startsWith(actual));
    });
  }

  function synchronizeRoute(path) {
    const next = cleanRoute(path);
    if (!route) route = next;
    else if (route === '/' && next !== '/') route = next;
    else if (next !== route) reset(next);
  }

  function liveMessages(messages) {
    return [...streams.values()].sort((left, right) => left.order - right.order)
      .filter((stream) => !authoritativeMatch(messages, stream))
      .map((stream) => ({
        id: stream.id,
        role: stream.role,
        state: stream.state,
        content: [{ type: stream.role === 'assistant' ? 'markdown' : 'text', text: stream.text }],
      }));
  }

  function mergeMessageWindow(windowValue, path) {
    const value = windowValue && typeof windowValue === 'object' ? windowValue : {};
    const messages = Array.isArray(value.messages) ? value.messages : [];
    synchronizeRoute(path);
    const live = liveMessages(messages);
    if (!live.length) return value;
    return {
      ...value,
      messages: messages.concat(live),
      observedCount: Math.max(Number(value.observedCount) || 0, messages.length + live.length),
    };
  }

  function reset(nextRoute) {
    streams.clear();
    seenHashes.clear();
    previousTextByStream = new Map();
    deltaDecoder = deltaFactory.create();
    route = cleanRoute(nextRoute || location.pathname);
    nextOrder = 0;
    revision += 1;
    scheduleStructuralState();
  }

  function decodeChannelData(value) {
    if (typeof value === 'string') return Promise.resolve(value);
    if (value instanceof ArrayBuffer) {
      if (value.byteLength > MAX_PAYLOAD_CHARS) return Promise.resolve('');
      try { return Promise.resolve(new TextDecoder('utf-8', { fatal: true }).decode(value)); }
      catch (_) { return Promise.resolve(''); }
    }
    if (typeof Blob !== 'undefined' && value instanceof Blob && value.size <= MAX_PAYLOAD_CHARS) {
      return value.arrayBuffer().then(decodeChannelData, () => '');
    }
    return Promise.resolve('');
  }

  function bindChannel(channel) {
    if (!channel || boundChannels.has(channel) || typeof channel.addEventListener !== 'function') return channel;
    boundChannels.add(channel);
    if (openChannels.size === 0) reset(location.pathname);
    openChannels.add(channel);
    if (channel.readyState === 'open') connectedChannels.add(channel);
    channel.addEventListener('open', () => {
      connectedChannels.add(channel);
      scheduleStructuralState();
    });
    channel.addEventListener('message', (event) => {
      void decodeChannelData(event && event.data).then((payload) => { if (payload) acceptPayload(payload); });
    });
    const close = () => {
      connectedChannels.delete(channel);
      openChannels.delete(channel);
      scheduleStructuralState();
    };
    channel.addEventListener('close', close);
    channel.addEventListener('error', close);
    scheduleStructuralState();
    return channel;
  }

  function hookPeer(peer) {
    if (!peer || boundPeers.has(peer)) return peer;
    boundPeers.add(peer);
    if (typeof peer.addEventListener === 'function') {
      peer.addEventListener('datachannel', (event) => bindChannel(event && event.channel));
    }
    if (typeof peer.createDataChannel === 'function') {
      const create = peer.createDataChannel.bind(peer);
      peer.createDataChannel = function (label, options) { return bindChannel(create(label, options)); };
    }
    return peer;
  }

  function wrapPeerConnection(name) {
    const NativePeerConnection = root[name];
    if (typeof NativePeerConnection !== 'function' || NativePeerConnection.__elonWinVoiceTranscriptWrapped) return;
    function WrappedPeerConnection(configuration, constraints) {
      return hookPeer(new NativePeerConnection(configuration, constraints));
    }
    WrappedPeerConnection.prototype = NativePeerConnection.prototype;
    Object.setPrototypeOf(WrappedPeerConnection, NativePeerConnection);
    Object.defineProperty(WrappedPeerConnection, '__elonWinVoiceTranscriptWrapped', { value: true });
    root[name] = WrappedPeerConnection;
  }

  const baseReadMessageWindow = messageAdapter.readMessageWindow.bind(messageAdapter);
  const baseReadMessages = messageAdapter.readMessages.bind(messageAdapter);
  const baseCapabilities = messageAdapter.capabilities.bind(messageAdapter);
  const enhancedMessageAdapter = Object.freeze({
    ...messageAdapter,
    capabilities() {
      return Array.from(new Set(baseCapabilities().concat('realtime_voice_private_transcript')));
    },
    readMessageWindow(streaming, streamingAssistantKey) {
      return mergeMessageWindow(baseReadMessageWindow(streaming, streamingAssistantKey), location.pathname);
    },
    readMessages(streaming) {
      const messages = baseReadMessages(streaming);
      return mergeMessageWindow({ messages, observedCount: messages.length, startIndex: 0 }, location.pathname).messages;
    },
  });

  root.__elonChatGptMessages = enhancedMessageAdapter;
  wrapPeerConnection('RTCPeerConnection');
  wrapPeerConnection('webkitRTCPeerConnection');
  root.__elonWinChatGptRealtimeVoiceTranscript = Object.freeze({
    version: 1,
    acceptPayload,
    decodePayload,
    hookPeer,
    mergeMessageWindow,
    reset,
    status() {
      return structuralState();
    },
  });
})();

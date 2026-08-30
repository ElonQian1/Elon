const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');
const vm = require('node:vm');

const root = path.resolve(__dirname, '..');
const deltaSource = fs.readFileSync(path.join(
  root, 'desktop-shell', 'src-tauri', 'src', 'local_ai_browser',
  'chatgpt_win_realtime_voice_json_delta.js',
), 'utf8');
const transcriptSource = fs.readFileSync(path.join(
  root, 'desktop-shell', 'src-tauri', 'src', 'local_ai_browser',
  'chatgpt_win_realtime_voice_transcript.js',
), 'utf8');

class EventTargetMock {
  constructor() { this.listeners = new Map(); }
  addEventListener(name, listener) {
    const values = this.listeners.get(name) || [];
    values.push(listener);
    this.listeners.set(name, values);
  }
  emit(name, value = {}) {
    for (const listener of this.listeners.get(name) || []) listener(value);
  }
}

class DataChannelMock extends EventTargetMock {
  constructor() {
    super();
    this.readyState = 'connecting';
  }
}
class PeerConnectionMock extends EventTargetMock {
  createDataChannel() {
    this.localChannel = new DataChannelMock();
    return this.localChannel;
  }
}

let baseMessages = [];
let snapshotRequests = 0;
const nativeEvents = [];
const windowValue = {
  RTCPeerConnection: PeerConnectionMock,
  setTimeout,
  clearTimeout,
  TextDecoder,
  ArrayBuffer,
  Blob,
  __elonChatGptDocumentToken: 'doc_win_voice_test',
  __elonChatGptAdapterVersion: 206,
  elonChatGptNative: {
    postMessage(payload) { nativeEvents.push(JSON.parse(payload)); },
  },
  __elonChatGptBridge: { command() { snapshotRequests += 1; } },
  __elonChatGptMessages: Object.freeze({
    capabilities: () => ['message_copy', 'rich_text'],
    readMessages: () => baseMessages,
    readMessageWindow: () => ({
      messages: baseMessages,
      observedCount: baseMessages.length,
      startIndex: 0,
    }),
  }),
};
const context = vm.createContext({
  window: windowValue,
  location: { origin: 'https://chatgpt.com', pathname: '/' },
  setTimeout,
  clearTimeout,
  TextDecoder,
  ArrayBuffer,
  Blob,
  console,
});
vm.runInContext(deltaSource, context, { filename: 'chatgpt_win_realtime_voice_json_delta.js' });
vm.runInContext(transcriptSource, context, { filename: 'chatgpt_win_realtime_voice_transcript.js' });

const runtime = windowValue.__elonWinChatGptRealtimeVoiceTranscript;
assert.equal(runtime.version, 1);
assert.match(windowValue.__elonChatGptMessages.capabilities().join(','), /realtime_voice_private_transcript/);

const privateMessage = (text, status) => ({
  id: 'voice_assistant_1',
  author: { role: 'assistant' },
  status,
  content: { content_type: 'text', parts: [text] },
});
const privateDelta = (delta) => JSON.stringify({
  type: 'data_message',
  data: JSON.stringify({
    type: 'chat_message_delta',
    payload: { type: 'chat_message_delta', delta },
  }),
});

assert.equal(runtime.acceptPayload(privateDelta({
  c: 0, o: 'add', p: '', v: { message: privateMessage('你', 'in_progress') },
})), true);
assert.equal(runtime.acceptPayload(privateDelta({
  o: 'append', p: '/message/content/parts/0', v: '好',
})), true);
assert.equal(runtime.acceptPayload(privateDelta({
  o: 'replace', p: '/message/status', v: 'finished_successfully',
})), true);

let messages = windowValue.__elonChatGptMessages.readMessageWindow(false, '').messages;
assert.equal(messages.length, 1);
assert.equal(messages[0].role, 'assistant');
assert.equal(messages[0].state, 'completed');
assert.equal(messages[0].content[0].text, '你好');

const userDelta = JSON.stringify({
  event_id: 'event_user_delta',
  type: 'conversation.item.input_audio_transcription.delta',
  item_id: 'voice_user_1',
  delta: '语音',
});
const userFinal = JSON.stringify({
  event_id: 'event_user_final',
  type: 'conversation.item.input_audio_transcription.completed',
  item_id: 'voice_user_1',
  transcript: '语音问题',
});
assert.equal(runtime.acceptPayload(userDelta), true);
assert.equal(runtime.acceptPayload(userDelta), false, 'duplicate event id must be ignored');
assert.equal(runtime.acceptPayload(userFinal), true);
messages = windowValue.__elonChatGptMessages.readMessageWindow(false, '').messages;
assert.equal(messages.length, 2);
assert.equal(messages[1].role, 'user');
assert.equal(messages[1].content[0].text, '语音问题');

baseMessages = [
  { id: 'official-assistant', role: 'assistant', state: 'completed', content: [{ type: 'markdown', text: '你好' }] },
  { id: 'official-user', role: 'user', state: 'completed', content: [{ type: 'text', text: '语音问题' }] },
];
messages = windowValue.__elonChatGptMessages.readMessageWindow(false, '').messages;
assert.equal(messages.length, 2, 'authoritative DOM messages must replace matching live previews');

assert.equal(runtime.acceptPayload('x'.repeat(256 * 1024 + 1)), false);
const structuralStatus = JSON.stringify(runtime.status());
assert.doesNotMatch(structuralStatus, /你好|语音问题/);
assert.equal(runtime.status().acceptedEventCount, 5);

const pollutionDecoder = windowValue.__elonWinChatGptRealtimeVoiceJsonDelta.create();
assert.equal(pollutionDecoder.apply({ c: 0, o: 'add', p: '/__proto__/polluted', v: true }), null);
assert.equal({}.polluted, undefined, 'private delta paths must not mutate object prototypes');

runtime.reset('/c/voice-channel');
baseMessages = [];
const peer = new windowValue.RTCPeerConnection();
const channel = peer.createDataChannel('');
channel.readyState = 'open';
channel.emit('open');
channel.emit('message', { data: JSON.stringify({
  event_id: 'channel_assistant_1',
  type: 'response.output_audio_transcript.done',
  item_id: 'channel_answer_1',
  transcript: '后台语音实时到达',
}) });

setTimeout(() => {
  const emptyWindow = { messages: [], observedCount: 0, startIndex: 0 };
  let channelMessages = runtime.mergeMessageWindow(emptyWindow, '/c/voice-channel').messages;
  assert.equal(channelMessages.length, 1);
  assert.equal(channelMessages[0].content[0].text, '后台语音实时到达');
  assert.equal(runtime.status().openChannelCount, 1);
  assert.ok(snapshotRequests > 0, 'accepted private transcript must request a native snapshot');
  const activeState = nativeEvents.map((entry) => entry.event).findLast((event) => (
    event && event.type === 'realtime_voice_state' && event.active === true
  ));
  assert.equal(activeState.openChannelCount, 1);
  assert.doesNotMatch(JSON.stringify(activeState), /后台语音实时到达/);

  channelMessages = runtime.mergeMessageWindow(emptyWindow, '/').messages;
  assert.equal(channelMessages.length, 0, 'new-chat navigation must clear old live voice text');
  assert.equal(runtime.status().openChannelCount, 1, 'route isolation must not duplicate audio channels');
  channel.readyState = 'closed';
  channel.emit('close');
  setTimeout(() => {
    const finalState = nativeEvents.map((entry) => entry.event).findLast((event) => (
      event && event.type === 'realtime_voice_state'
    ));
    assert.equal(finalState.active, false);
    assert.equal(finalState.openChannelCount, 0);
    process.stdout.write('PASS Win ChatGPT realtime voice private transcript bridge\n');
  }, 60);
}, 90);

'use strict';

const assert = require('assert');
const fs = require('fs');
const path = require('path');
const vm = require('vm');

const root = path.resolve(__dirname, '..');
const source = fs.readFileSync(
  path.join(root, 'android', 'app', 'src', 'main', 'assets', 'chatgpt_web_private_voice_relay.js'),
  'utf8'
);
const buildGradle = fs.readFileSync(
  path.join(root, 'android', 'app', 'build.gradle'),
  'utf8'
);
const pageAdapter = fs.readFileSync(
  path.join(root, 'android', 'app', 'src', 'main', 'kotlin', 'com', 'elon', 'app',
    'chatgptweb', 'ChatGptWebPageAdapter.kt'),
  'utf8'
);
const relayGateway = fs.readFileSync(
  path.join(root, 'android', 'app', 'src', 'main', 'kotlin', 'com', 'elon', 'app',
    'chatgptweb', 'ChatGptWebPrivateVoiceRelayGateway.kt'),
  'utf8'
);

assert.match(
  buildGradle,
  /chatGptPrivateVoiceNativeRtcProperty == null[\s\S]*?\? true[\s\S]*?: chatGptPrivateVoiceNativeRtcProperty\.toString\(\)\.toBoolean\(\)/
);
assert.doesNotMatch(
  buildGradle,
  /ELON_CHATGPT_PRIVATE_VOICE_NATIVE_RTC requires ELON_CHATGPT_PRIVATE_RESEARCH=true/
);
assert.match(
  pageAdapter,
  /window\.__elonChatGptPrivateVoiceNativeRtcEnabled =[\s\S]*?BuildConfig\.CHATGPT_PRIVATE_VOICE_NATIVE_RTC_ENABLED/
);
assert.match(
  pageAdapter,
  /BuildConfig\.CHATGPT_PRIVATE_RESEARCH_ENABLED \|\|[\s\S]*?BuildConfig\.CHATGPT_PRIVATE_VOICE_NATIVE_RTC_ENABLED/
);
assert.match(
  relayGateway,
  /if \(!BuildConfig\.CHATGPT_PRIVATE_VOICE_NATIVE_RTC_ENABLED\)/
);
const nativeAnswer = 'v=0\r\nm=audio 9 UDP/TLS/RTP/SAVPF 111\r\n';
const officialAnswer = 'v=0\r\nm=audio 8 UDP/TLS/RTP/SAVPF 111\r\n';
const originalOffer = 'v=0\r\nm=audio 1 UDP/TLS/RTP/SAVPF 111\r\n';
const nativeOffer = 'v=0\r\nm=audio 2 UDP/TLS/RTP/SAVPF 111\r\n';

function createHarness(responseForCall, featureFlags = {}) {
  const calls = [];
  const upstream = async (input, init) => {
    calls.push({ input, init });
    return responseForCall(calls.length, input, init);
  };
  class FakePeerConnection {
    constructor() {
      this.senderTrack = { kind: 'audio', readyState: 'live', enabled: true };
      this.receiverTrack = { kind: 'audio', readyState: 'live', enabled: true };
      this.closed = false;
      this.remoteDescriptions = [];
    }
    createDataChannel(label, options) {
      return { label, options };
    }
    addTrack(track) {
      this.senderTrack = track;
      return { track, replaceTrack: (next) => { this.senderTrack = next; } };
    }
    getSenders() {
      return [{ track: this.senderTrack }];
    }
    getReceivers() {
      return [{ track: this.receiverTrack }];
    }
    setRemoteDescription(description) {
      this.remoteDescriptions.push(description);
      return Promise.resolve();
    }
    close() {
      this.closed = true;
    }
  }
  const window = {
    __elonChatGptPrivateResearchEnabled: featureFlags.research === true,
    __elonChatGptPrivateVoiceNativeRtcEnabled: featureFlags.nativeRtc !== false,
    fetch: upstream,
    RTCPeerConnection: FakePeerConnection
  };
  window.window = window;
  vm.runInNewContext(source, {
    window,
    location: { origin: 'https://chatgpt.com', href: 'https://chatgpt.com/' },
    URL,
    Headers,
    FormData,
    Blob,
    Response,
    Date,
    Map,
    JSON,
    Promise,
    AbortController,
    setTimeout,
    clearTimeout
  }, { filename: 'chatgpt_web_private_voice_relay.js' });
  return { window, calls };
}

function officialRequest(privateSession) {
  const body = new FormData();
  body.append('sdp', originalOffer);
  body.append('session', privateSession);
  return {
    method: 'POST',
    credentials: 'include',
    headers: { 'x-private-runtime': 'private-must-stay-in-page-memory' },
    body
  };
}

async function take(window, requestId) {
  for (let index = 0; index < 20; index += 1) {
    const value = window.__elonChatGptPrivateVoiceRelay.takeResult(requestId);
    if (value) return JSON.parse(value);
    await new Promise((resolve) => setTimeout(resolve, 0));
  }
  throw new Error(`No relay result for ${requestId}`);
}

async function verifiesAtomicTakeover() {
  const { window, calls } = createHarness(async () =>
    new Response(nativeAnswer, { status: 201, headers: { 'content-type': 'application/sdp' } })
  );
  const relay = window.__elonChatGptPrivateVoiceRelay;
  assert.equal(relay.version, 4);
  assert.deepEqual(JSON.parse(relay.bootstrap()).dataChannel, {
    label: '',
    ordered: true,
    maxRetransmits: null,
    protocol: '',
    negotiated: false,
    id: null
  });
  assert.equal(JSON.parse(relay.bootstrap()).dataChannelState, 'preset');
  assert.deepEqual(JSON.parse(relay.armExchange('relay_12345678', nativeOffer)), {
    version: 4,
    armed: true,
    code: null
  });

  const peer = new window.RTCPeerConnection();
  const lateTrack = { kind: 'audio', readyState: 'live', enabled: true };
  peer.addTrack(lateTrack);
  peer.createDataChannel('', { ordered: true, protocol: '', negotiated: false });
  assert.equal(lateTrack.enabled, false);
  assert.equal(peer.receiverTrack.enabled, false);

  const privateSession = JSON.stringify({
    conversation_id: 'private-must-stay-in-page-memory',
    backend_model: 'private-must-stay-in-page-memory'
  });
  const response = await window.fetch(
    'https://chatgpt.com/realtime/wm',
    officialRequest(privateSession)
  );
  assert.equal(await response.text(), nativeAnswer);
  assert.equal(calls.length, 1);
  assert.equal(calls[0].init.body.get('sdp'), nativeOffer);
  assert.equal(calls[0].init.body.get('session'), privateSession);
  assert.equal(calls[0].init.credentials, 'include');
  assert.ok(calls[0].init.signal instanceof AbortSignal);
  assert.equal(calls[0].init.headers.get('x-private-runtime'), 'private-must-stay-in-page-memory');
  assert.deepEqual(await take(window, 'relay_12345678'), {
    status: 'ok',
    answer: nativeAnswer
  });

  await peer.setRemoteDescription({ type: 'answer', sdp: nativeAnswer });
  assert.equal(peer.remoteDescriptions.length, 0);
  const replay = await window.fetch(
    'https://chatgpt.com/realtime/wm',
    officialRequest(privateSession)
  );
  assert.equal(await replay.text(), nativeAnswer);
  assert.equal(calls.length, 1);

  const replacementPeer = new window.RTCPeerConnection();
  replacementPeer.createDataChannel('', { ordered: true, protocol: '' });
  assert.equal(peer.closed, true);
  assert.equal(replacementPeer.senderTrack.enabled, false);
  assert.equal(replacementPeer.receiverTrack.enabled, false);

  assert.deepEqual(JSON.parse(relay.resetTakeover()), {
    version: 4,
    applied: true,
    enabled: true,
    senderTracks: 1,
    receiverTracks: 1,
    closed: true
  });
  assert.equal(replacementPeer.closed, true);
  const nextPeer = new window.RTCPeerConnection();
  nextPeer.createDataChannel('', { ordered: true, protocol: '' });
  assert.equal(nextPeer.senderTrack.enabled, true);
  assert.equal(nextPeer.receiverTrack.enabled, true);

  const publicState = `${relay.state()} ${relay.bootstrap()}`.toLowerCase();
  assert.doesNotMatch(publicState, /private-must-stay|conversation_id|backend_model|x-private-runtime/);
}

async function verifiesOfficialFallback() {
  const { window, calls } = createHarness(async (callNumber) => {
    if (callNumber === 1) return new Response('not-an-answer', { status: 201 });
    return new Response(officialAnswer, {
      status: 201,
      headers: { 'content-type': 'application/sdp' }
    });
  });
  const relay = window.__elonChatGptPrivateVoiceRelay;
  assert.equal(JSON.parse(relay.armExchange('relay_abcdefgh', nativeOffer)).armed, true);
  const peer = new window.RTCPeerConnection();
  peer.createDataChannel('', { ordered: true, protocol: '' });
  const privateSession = JSON.stringify({ conversation_id: 'private-fallback' });
  const response = await window.fetch(
    'https://chatgpt.com/realtime/wm',
    officialRequest(privateSession)
  );
  assert.equal(await response.text(), officialAnswer);
  assert.equal(calls.length, 2);
  assert.equal(calls[0].init.body.get('sdp'), nativeOffer);
  assert.equal(calls[1].init.body.get('sdp'), originalOffer);
  assert.deepEqual(await take(window, 'relay_abcdefgh'), {
    status: 'failed',
    code: 'invalid_answer'
  });
  assert.equal(peer.senderTrack.enabled, true);
  assert.equal(peer.receiverTrack.enabled, true);
  await peer.setRemoteDescription({ type: 'answer', sdp: officialAnswer });
  assert.equal(peer.remoteDescriptions.length, 1);
}

async function verifiesDedicatedCapabilityGate() {
  const upstream = async () => new Response(officialAnswer, { status: 201 });
  const { window } = createHarness(upstream, { research: false, nativeRtc: false });
  assert.equal(window.__elonChatGptPrivateVoiceRelay, undefined);
  const response = await window.fetch('https://chatgpt.com/realtime/wm', {});
  assert.equal(await response.text(), officialAnswer);
}

async function main() {
  await verifiesAtomicTakeover();
  await verifiesOfficialFallback();
  await verifiesDedicatedCapabilityGate();
  console.log('CHATGPT_WEB_PRIVATE_VOICE_RELAY_TESTS=passed');
}

main().catch((error) => {
  console.error(error);
  process.exitCode = 1;
});

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
const calls = [];
const answer = 'v=0\r\nm=audio 9 UDP/TLS/RTP/SAVPF 111\r\n';
const upstream = async (input, init) => {
  calls.push({ input, init });
  return new Response(answer, { status: 201, headers: { 'content-type': 'application/sdp' } });
};
const window = {
  __elonChatGptPrivateResearchEnabled: true,
  fetch: upstream
};
window.window = window;
const context = {
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
  setTimeout,
  clearTimeout
};
vm.runInNewContext(source, context, { filename: 'chatgpt_web_private_voice_relay.js' });

async function take(requestId) {
  for (let index = 0; index < 20; index += 1) {
    const value = window.__elonChatGptPrivateVoiceRelay.takeResult(requestId);
    if (value) return JSON.parse(value);
    await new Promise((resolve) => setTimeout(resolve, 0));
  }
  throw new Error(`No relay result for ${requestId}`);
}

async function main() {
  const originalOffer = 'v=0\r\nm=audio 1 UDP/TLS/RTP/SAVPF 111\r\n';
  const nativeOffer = 'v=0\r\nm=audio 2 UDP/TLS/RTP/SAVPF 111\r\n';
  const privateSession = JSON.stringify({
    conversation_id: 'private-must-stay-in-page-memory',
    backend_model: 'private-must-stay-in-page-memory'
  });
  const officialBody = new FormData();
  officialBody.append('sdp', originalOffer);
  officialBody.append('session', privateSession);

  await window.fetch('https://chatgpt.com/realtime/wm', {
    method: 'POST',
    credentials: 'include',
    headers: { 'x-private-runtime': 'private-must-stay-in-page-memory' },
    body: officialBody
  });

  const relay = window.__elonChatGptPrivateVoiceRelay;
  assert.equal(relay.version, 1);
  assert.deepEqual(JSON.parse(relay.state()), {
    version: 1,
    available: true,
    templateGeneration: 1,
    templateState: 'ready',
    inFlight: false
  });
  assert.equal(relay.startExchange('relay_12345678', nativeOffer), true);
  const result = await take('relay_12345678');
  assert.equal(result.status, 'ok');
  assert.equal(result.answer, answer);
  assert.equal(calls.length, 2);
  assert.equal(calls[1].init.body.get('sdp'), nativeOffer);
  assert.equal(calls[1].init.body.get('session'), privateSession);
  assert.equal(calls[1].init.credentials, 'include');
  assert.equal(calls[1].init.headers.get('x-private-runtime'), 'private-must-stay-in-page-memory');

  assert.equal(relay.startExchange('relay_abcdefgh', nativeOffer), false);
  assert.deepEqual(await take('relay_abcdefgh'), {
    status: 'failed',
    code: 'template_consumed'
  });
  assert.equal(relay.startExchange('relay_invalid1', 'not-an-offer'), false);
  assert.deepEqual(await take('relay_invalid1'), {
    status: 'failed',
    code: 'invalid_offer'
  });

  const publicState = relay.state().toLowerCase();
  assert.doesNotMatch(publicState, /private-must-stay|conversation_id|backend_model|x-private-runtime/);
  console.log('CHATGPT_WEB_PRIVATE_VOICE_RELAY_TESTS=passed');
}

main().catch((error) => {
  console.error(error);
  process.exitCode = 1;
});

'use strict';

const fs = require('fs');

const endpoint = process.argv[2] || 'http://127.0.0.1:9222';
const audioPath = process.argv[3];
const restoreOnly = audioPath === '--restore';

if (!audioPath) {
  throw new Error(
    'Usage: node inject-chatgpt-web-synthetic-microphone.js <cdp-endpoint> <wav-path|--restore>'
  );
}

async function selectTarget() {
  const response = await fetch(`${endpoint.replace(/\/$/, '')}/json`);
  if (!response.ok) throw new Error(`CDP target listing failed: ${response.status}`);
  const targets = await response.json();
  const target = targets.find((entry) => {
    try {
      return new URL(entry.url).hostname === 'chatgpt.com' && entry.webSocketDebuggerUrl;
    } catch (_) {
      return false;
    }
  });
  if (!target) throw new Error('No ChatGPT WebView target is available');
  return target;
}

function evaluate(webSocketUrl, expression) {
  return new Promise((resolve, reject) => {
    const socket = new WebSocket(webSocketUrl);
    const timeout = setTimeout(() => {
      socket.close();
      reject(new Error('CDP evaluation timed out'));
    }, 15000);

    socket.addEventListener('open', () => {
      socket.send(JSON.stringify({
        id: 1,
        method: 'Runtime.evaluate',
        params: {
          expression,
          awaitPromise: true,
          returnByValue: true
        }
      }));
    });
    socket.addEventListener('message', (event) => {
      const message = JSON.parse(String(event.data));
      if (message.id !== 1) return;
      clearTimeout(timeout);
      socket.close();
      const details = message.result && message.result.exceptionDetails;
      if (details) {
        reject(new Error(details.text || 'CDP evaluation failed'));
        return;
      }
      resolve(message.result && message.result.result && message.result.result.value);
    });
    socket.addEventListener('error', () => {
      clearTimeout(timeout);
      reject(new Error('CDP WebSocket failed'));
    });
  });
}

async function main() {
  const target = await selectTarget();
  if (restoreOnly) {
    const restored = await evaluate(target.webSocketDebuggerUrl, `
      (async () => {
        const fixture = window.__elonSyntheticMicrophone;
        if (!fixture) return { restored: true, existed: false };
        navigator.mediaDevices.getUserMedia = fixture.original;
        try { fixture.source.stop(); } catch (_) {}
        try { await fixture.context.close(); } catch (_) {}
        delete window.__elonSyntheticMicrophone;
        return { restored: true, existed: true };
      })()
    `);
    if (!restored || restored.restored !== true) {
      throw new Error('Synthetic microphone was not restored');
    }
    process.stdout.write(JSON.stringify({ ok: true, restored: true }));
    return;
  }
  const audioBase64 = fs.readFileSync(audioPath).toString('base64');
  const expression = `
    (async () => {
      const binary = atob(${JSON.stringify(audioBase64)});
      const bytes = new Uint8Array(binary.length);
      for (let index = 0; index < binary.length; index += 1) {
        bytes[index] = binary.charCodeAt(index);
      }
      const AudioContextClass = window.AudioContext || window.webkitAudioContext;
      if (!AudioContextClass) throw new Error('audio_context_unavailable');
      const context = new AudioContextClass();
      const buffer = await context.decodeAudioData(bytes.buffer);
      const destination = context.createMediaStreamDestination();
      const source = context.createBufferSource();
      source.buffer = buffer;
      source.connect(destination);
      const original = navigator.mediaDevices.getUserMedia.bind(navigator.mediaDevices);
      let consumed = false;
      navigator.mediaDevices.getUserMedia = async (constraints) => {
        if (!constraints || !constraints.audio || consumed) return original(constraints);
        consumed = true;
        await context.resume();
        source.start(context.currentTime + 0.35);
        return destination.stream;
      };
      window.__elonSyntheticMicrophone = { context, destination, source, original };
      return { armed: true, duration_ms: Math.round(buffer.duration * 1000) };
    })()
  `;
  const result = await evaluate(target.webSocketDebuggerUrl, expression);
  if (!result || result.armed !== true) throw new Error('Synthetic microphone was not armed');
  process.stdout.write(JSON.stringify({ ok: true, duration_ms: result.duration_ms }));
}

main().catch((error) => {
  process.stderr.write(`${error.message}\n`);
  process.exitCode = 1;
});

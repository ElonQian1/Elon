import test from 'node:test';
import assert from 'node:assert/strict';
import { ScanCamera, decodeFrame, decodeImage, fetchScanImage } from './browserScanner.mjs';

const frame = () => ({ width: 1, height: 1, data: new Uint8ClampedArray(4) });
function mockGlobal(t, name, value) {
  const descriptor = Object.getOwnPropertyDescriptor(globalThis, name);
  Object.defineProperty(globalThis, name, { configurable: true, value });
  t.after(() => { if (descriptor) Object.defineProperty(globalThis, name, descriptor); else delete globalThis[name]; });
}
test('worker results deduplicate and every completion releases the worker', async () => {
  let stopped = 0;
  const worker = { postMessage() { queueMicrotask(() => this.onmessage({ data: ['a', 'a', '', 'b'] })); }, terminate() { stopped++; } };
  assert.deepEqual(await decodeFrame(frame(), () => worker), ['a', 'b']);
  assert.equal(stopped, 1);
});
test('cancellation terminates decoder and ignores a late reply', async () => {
  const controller = new AbortController(); let stopped = 0;
  const worker = { postMessage() {}, terminate() { stopped++; } };
  const pending = decodeFrame(frame(), () => worker, controller.signal);
  controller.abort(); worker.onmessage({ data: ['late'] });
  await assert.rejects(pending, { name: 'AbortError' }); assert.equal(stopped, 1);
});
test('image input rejects non-images and oversized files before allocating', async () => {
  const fail = () => { throw Error('must not start'); };
  await assert.rejects(decodeImage({ type: 'text/plain', size: 12 }, fail), /图片文件/);
  await assert.rejects(decodeImage({ type: 'image/png', size: 13 * 1024 * 1024 }, fail), /12 MB/);
});
test('camera closes a stream granted after scanner has closed', async t => {
  let grant, stopped = 0;
  mockGlobal(t, 'isSecureContext', true);
  mockGlobal(t, 'navigator', { mediaDevices: { getUserMedia: () => new Promise(resolve => { grant = resolve; }) } });
  const video = { srcObject: null, play: async () => {}, readyState: 0 };
  const camera = new ScanCamera(video, () => {}, () => assert.fail('unexpected result'), () => {});
  const pending = camera.start(); camera.stop();
  grant({ getTracks: () => [{ stop() { stopped++; } }] }); await pending;
  assert.equal(stopped, 1); assert.equal(video.srcObject, null);
});
test('camera denial remains recoverable and stop releases an active stream', async t => {
  let stopped = 0, denied = true; const messages = [];
  const track = { stop() { stopped++; }, addEventListener() {}, getCapabilities: () => ({ torch: true }), applyConstraints: async () => {} };
  mockGlobal(t, 'isSecureContext', true);
  mockGlobal(t, 'navigator', { mediaDevices: { getUserMedia: async () => {
    if (denied) throw new DOMException('denied', 'NotAllowedError');
    return { getTracks: () => [track], getVideoTracks: () => [track] };
  } } });
  const video = { srcObject: null, play: async () => {}, readyState: 0 };
  const camera = new ScanCamera(video, () => {}, () => {}, value => messages.push(value));
  await camera.start(); assert.match(messages.at(-1), /未获授权/);
  denied = false; await camera.start(); assert.equal(camera.canTorch(), true);
  await camera.torch(true); camera.stop(); assert.equal(stopped, 1); assert.equal(video.srcObject, null);
});
test('insecure context uses image fallback without requesting the camera', async t => {
  mockGlobal(t, 'isSecureContext', false);
  const messages = [], camera = new ScanCamera({}, () => {}, () => {}, message => messages.push(message));
  await camera.start(); assert.match(messages[0], /HTTPS/);
});
test('streaming image download enforces size without buffering entire response', async t => {
  mockGlobal(t, 'location', new URL('https://example.com'));
  let canceled = false;
  t.mock.method(globalThis, 'fetch', async () => ({ ok: true, headers: new Headers(), body: { getReader: () => ({
    read: async () => ({ done: false, value: new Uint8Array(13 * 1024 * 1024) }),
    cancel: async () => { canceled = true; }, releaseLock() {},
  }) } }));
  await assert.rejects(fetchScanImage('/image'), /12 MB/); assert.equal(canceled, true);
});

export const MAX_IMAGE_BYTES = 12 * 1024 * 1024;
const abortError = () => new DOMException('识别已取消', 'AbortError');
export function decodeFrame(frame, workerFactory, signal) {
  return new Promise((resolve, reject) => {
    if (signal?.aborted) return reject(abortError());
    const worker = workerFactory();
    let settled = false;
    const done = (error, values) => {
      if (settled) return; settled = true;
      clearTimeout(timer); worker.terminate(); signal?.removeEventListener('abort', cancel);
      error ? reject(error) : resolve([...new Set(values.filter(v => typeof v === 'string' && v.trim() && v.length <= 8192))].slice(0, 8));
    };
    const cancel = () => done(abortError());
    const timer = setTimeout(() => done(new Error('二维码识别超时，请尝试清晰原图')), 8000);
    signal?.addEventListener('abort', cancel, { once: true });
    worker.onmessage = event => Array.isArray(event.data) ? done(null, event.data) : done(new Error('二维码识别结果异常'));
    worker.onerror = () => done(new Error('二维码识别失败，请重新选择图片'));
    try { worker.postMessage({ pixels: frame.data, width: frame.width, height: frame.height }, [frame.data.buffer]); }
    catch (error) { done(error); }
  });
}

function frameFrom(source, width, height, maxSide = 2000) {
  if (!width || !height || width * height > 80_000_000) throw new Error('图片尺寸过大或无法读取');
  const scale = Math.min(1, maxSide / Math.max(width, height));
  const canvas = document.createElement('canvas');
  canvas.width = Math.max(1, Math.round(width * scale)); canvas.height = Math.max(1, Math.round(height * scale));
  const context = canvas.getContext('2d', { willReadFrequently: true });
  if (!context) throw new Error('当前浏览器无法读取图片，请换用系统浏览器');
  context.drawImage(source, 0, 0, canvas.width, canvas.height);
  return context.getImageData(0, 0, canvas.width, canvas.height);
}

export async function decodeImage(blob, workerFactory, signal) {
  if (!blob || blob.size > MAX_IMAGE_BYTES) throw new Error('请选择不超过 12 MB 的图片');
  if (!blob.type.startsWith('image/')) throw new Error('请选择图片文件');
  let image, objectUrl;
  try {
    if (typeof createImageBitmap === 'function') image = await createImageBitmap(blob);
    else {
      objectUrl = URL.createObjectURL(blob); image = new Image();
      await new Promise((resolve, reject) => { image.onload = resolve; image.onerror = () => reject(new Error('图片无法解码')); image.src = objectUrl; });
    }
    if (signal?.aborted) throw abortError();
    const width = image.width, height = image.height;
    const found = await decodeFrame(frameFrom(image, width, height), workerFactory, signal);
    // Large screenshots can contain small symbols; retry at a bounded higher resolution.
    return found.length || Math.max(width, height) <= 2000 ? found
      : decodeFrame(frameFrom(image, width, height, 3200), workerFactory, signal);
  } finally { image?.close?.(); if (objectUrl) URL.revokeObjectURL(objectUrl); }
}

export async function fetchScanImage(url, signal) {
  const parsed = new URL(url, location.href);
  if (!['http:', 'https:', 'blob:', 'data:'].includes(parsed.protocol)) throw new Error('图片地址不支持，请保存后从文件选择');
  const response = await fetch(url, { signal, credentials: parsed.origin === location.origin ? 'same-origin' : 'omit' });
  if (!response.ok) throw new Error('图片不可读取，请保存原图后从文件选择');
  if (Number(response.headers.get('content-length')) > MAX_IMAGE_BYTES) throw new Error('图片超过 12 MB');
  const reader = response.body?.getReader();
  if (!reader) throw new Error('当前浏览器无法读取该图片，请从文件选择');
  const chunks = []; let size = 0;
  try {
    while (true) {
      const { done, value } = await reader.read(); if (done) break;
      size += value.byteLength;
      if (size > MAX_IMAGE_BYTES) throw new Error('图片超过 12 MB');
      chunks.push(value);
    }
    return new Blob(chunks, { type: response.headers.get('content-type')?.split(';')[0] || 'image/png' });
  } finally { await reader.cancel().catch(() => {}); reader.releaseLock(); }
}

export function cameraErrorMessage(error) {
  if (['NotAllowedError', 'SecurityError'].includes(error?.name)) return '相机未获授权，请在浏览器设置中允许相机，或选择图片识别';
  if (error?.name === 'NotFoundError') return '未找到摄像头，请选择图片识别';
  if (error?.name === 'NotReadableError') return '相机被占用，请关闭其他相机应用后重试';
  return error?.message || '相机无法启动，请选择图片识别';
}

export class ScanCamera {
  constructor(video, workerFactory, onResults, onState) {
    Object.assign(this, { video, workerFactory, onResults, onState });
    this.revision = 0; this.facing = 'environment'; this.stream = null;
  }
  stop() {
    ++this.revision; clearTimeout(this.timer); clearTimeout(this.permissionTimer);
    this.abort?.abort(); this.stream?.getTracks().forEach(track => track.stop());
    this.stream = null; this.video.srcObject = null;
  }
  async start(facing = this.facing) {
    this.stop(); this.facing = facing; const revision = this.revision;
    if (!globalThis.isSecureContext || !navigator.mediaDevices?.getUserMedia) {
      this.onState('相机扫码需要 HTTPS 和支持相机的浏览器，仍可选择图片识别'); return;
    }
    this.onState('正在请求相机权限…');
    this.permissionTimer = setTimeout(() => {
      if (revision === this.revision && !this.stream) { this.stop(); this.onState('等待相机授权超时，请重试或选择图片'); }
    }, 15000);
    try {
      const stream = await navigator.mediaDevices.getUserMedia({ audio: false, video: { facingMode: { ideal: facing }, width: { ideal: 1280 }, height: { ideal: 720 } } });
      if (revision !== this.revision) { stream.getTracks().forEach(track => track.stop()); return; }
      clearTimeout(this.permissionTimer); this.stream = stream; this.video.srcObject = stream;
      stream.getVideoTracks().forEach(track => track.addEventListener('ended', () => {
        if (revision === this.revision) { this.stop(); this.onState('相机已断开，请重试或选择图片'); }
      }, { once: true }));
      await this.video.play(); if (revision !== this.revision) return;
      this.abort = new AbortController(); this.onState('将二维码放入画面，识别成功后会暂停'); this.tick(revision);
    } catch (error) { if (revision === this.revision) { this.stop(); this.onState(cameraErrorMessage(error)); } }
  }
  async tick(revision) {
    if (revision !== this.revision) return;
    try {
      if (this.video.readyState >= 2 && this.video.videoWidth) {
        const frame = frameFrom(this.video, this.video.videoWidth, this.video.videoHeight, 1280);
        const values = await decodeFrame(frame, this.workerFactory, this.abort.signal);
        if (revision !== this.revision) return;
        if (values.length) { this.stop(); this.onResults(values); return; }
      }
      if (revision === this.revision) this.timer = setTimeout(() => this.tick(revision), 250);
    } catch (error) { if (revision === this.revision) { this.stop(); this.onState(error.message); } }
  }
  canTorch() { return Boolean(this.stream?.getVideoTracks()[0]?.getCapabilities?.().torch); }
  async torch(enabled) {
    const track = this.stream?.getVideoTracks()[0];
    if (!this.canTorch()) throw new Error('当前相机不支持手电筒');
    await track.applyConstraints({ advanced: [{ torch: enabled }] });
  }
}

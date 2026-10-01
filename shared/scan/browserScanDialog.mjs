import { ScanCamera, decodeImage, fetchScanImage } from './browserScanner.mjs';
import { parseScanPayload } from './scanPayload.mjs';

// One local-only result flow for PWA and desktop. All navigation needs a user action.
export function openScanDialog({ workerFactory, onFriend, imageUrl, onClose = () => {} }) {
  const origin = document.activeElement;
  const dialog = document.createElement('dialog'); dialog.className = 'elon-scan-dialog';
  dialog.setAttribute('aria-label', '扫一扫');
  dialog.innerHTML = `<header><h2>扫一扫</h2><button type="button" data-close aria-label="关闭扫一扫">关闭</button></header>
    <p class="elon-scan-hint">相机或图片中的二维码 · 在本机识别</p>
    <video playsinline muted aria-label="扫码相机预览"></video>
    <div class="elon-scan-controls"><button type="button" data-camera>开启相机</button><button type="button" data-file>选择图片</button>
    <button type="button" data-switch hidden>切换镜头</button><button type="button" data-torch hidden aria-pressed="false">手电筒</button></div>
    <input type="file" accept="image/*" hidden><p class="elon-scan-status" role="status" aria-live="polite"></p><section class="elon-scan-results" aria-label="识别结果"></section>`;
  const get = selector => dialog.querySelector(selector);
  const video = get('video'), status = get('[role=status]'), results = get('section'), file = get('input');
  const torchButton = get('[data-torch]'), switchButton = get('[data-switch]');
  let closed = false, torch = false, request = 0, abort, facing = 'environment';
  const camera = new ScanCamera(video, workerFactory, showResults, message => {
    if (closed) return;
    status.textContent = message; video.hidden = !camera.stream;
    torchButton.hidden = !camera.canTorch(); switchButton.hidden = !camera.stream;
  });
  function stop() {
    ++request; abort?.abort(); camera.stop(); torch = false;
    torchButton.setAttribute('aria-pressed', 'false'); torchButton.hidden = true; switchButton.hidden = true; video.hidden = true;
  }
  function close() {
    if (closed) return; closed = true; stop();
    document.removeEventListener('visibilitychange', hidden); window.removeEventListener('pagehide', close);
    dialog.close(); dialog.remove(); if (origin instanceof HTMLElement && origin.isConnected) origin.focus(); onClose();
  }
  function hidden() { if (document.hidden) { stop(); status.textContent = '相机已暂停，返回后可继续扫描'; } }
  function showResults(values) {
    if (closed) return; stop(); results.replaceChildren();
    const parsed = [...new Set(values)].flatMap(value => { try { return [parseScanPayload(value)]; } catch { return []; } }).slice(0, 8);
    status.textContent = parsed.length ? `识别到 ${parsed.length} 个二维码，请选择操作` : '没有识别到二维码，请换用清晰原图或靠近后重试';
    get('[data-camera]').textContent = '继续扫描';
    for (const result of parsed) {
      const card = document.createElement('article'), title = document.createElement('h3'), text = document.createElement('pre');
      title.textContent = result.title; text.textContent = result.raw; card.append(title, text);
      const copy = document.createElement('button'); copy.type = 'button'; copy.textContent = '复制内容';
      copy.onclick = async () => {
        try { await navigator.clipboard.writeText(result.raw); status.textContent = '已复制'; }
        catch { status.textContent = '复制未获授权，可选中内容手动复制'; }
      }; card.append(copy);
      if (result.kind === 'friend') {
        const action = document.createElement('button'); action.type = 'button'; action.textContent = '查找此账号';
        action.onclick = () => { close(); onFriend(result.target); }; card.append(action);
      } else if (result.target) {
        const action = document.createElement('a'); action.href = result.target;
        action.textContent = { url: '打开网页', phone: '打开拨号', sms: '编辑短信', email: '撰写邮件', geo: '打开地图' }[result.kind];
        if (result.kind === 'url') { action.target = '_blank'; action.rel = 'noopener noreferrer'; }
        card.append(action);
      }
      results.append(card);
    }
  }
  async function scanImage(source) {
    stop(); results.replaceChildren(); const revision = request, controller = new AbortController(); abort = controller;
    const timer = setTimeout(() => controller.abort(), 20000);
    status.textContent = '正在本地识别图片…';
    try {
      const blob = typeof source === 'string' ? await fetchScanImage(source, controller.signal) : source;
      const values = await decodeImage(blob, workerFactory, controller.signal);
      if (!closed && revision === request) showResults(values);
    } catch (error) {
      if (!closed && revision === request) status.textContent = error.name === 'AbortError' ? '读取图片超时，请保存原图后选择图片' : error.name === 'TypeError' ? '图片无法读取，请保存原图后从文件选择' : error.message;
    } finally { clearTimeout(timer); }
  }
  async function startCamera() {
    stop(); results.replaceChildren(); await camera.start(facing);
  }
  get('[data-close]').onclick = close;
  dialog.oncancel = event => { event.preventDefault(); close(); };
  dialog.addEventListener('keydown', event => event.stopPropagation());
  get('[data-camera]').onclick = () => void startCamera();
  get('[data-file]').onclick = () => { stop(); status.textContent = '选择一张含二维码的图片（不超过 12 MB）'; file.click(); };
  file.onchange = () => { const selected = file.files?.[0]; file.value = ''; if (selected) void scanImage(selected); };
  switchButton.onclick = () => { facing = facing === 'environment' ? 'user' : 'environment'; void startCamera(); };
  torchButton.onclick = async () => {
    try { await camera.torch(!torch); torch = !torch; torchButton.setAttribute('aria-pressed', String(torch)); }
    catch { status.textContent = '手电筒无法开启，可调整环境光线后重试'; }
  };
  document.addEventListener('visibilitychange', hidden); window.addEventListener('pagehide', close);
  document.body.append(dialog); dialog.showModal(); video.hidden = true;
  if (imageUrl) void scanImage(imageUrl); else status.textContent = '开启相机或选择图片，识别后由你确认操作';
  return { close };
}

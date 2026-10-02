(function (root) {
  'use strict';
  let current = null, returning = false;
  const historyKey = 'elonImagePreview';
  const clamp = (value, min, max) => Math.max(min, Math.min(max, value));
  function element(tag, className, text) {
    const node = document.createElement(tag); node.className = className;
    if (text) node.textContent = text;
    return node;
  }
  function button(label, text, action) {
    const node = element('button', '', text); node.type = 'button';
    node.setAttribute('aria-label', label); node.title = label; node.onclick = action;
    return node;
  }
  function transform(stage, image, update) {
    const view = { scale: 1, x: 0, y: 0, width: 0, height: 0, ready: false, reading: false, widthScale: 1, maxScale: 8, stageHeight: 0 };
    function paint() {
      const maxX = Math.max(0, (view.width * view.scale - stage.clientWidth) / 2);
      const maxY = Math.max(0, (view.height * view.scale - stage.clientHeight) / 2);
      view.x = clamp(view.x, -maxX, maxX); view.y = clamp(view.y, -maxY, maxY);
      image.style.transform = `translate(${view.x}px, ${view.y}px) scale(${view.scale})`;
      stage.dataset.scale = String(view.scale); stage.dataset.reading = String(view.reading); update(view);
    }
    function fit(initial = false) {
      if (!image.naturalWidth || !image.naturalHeight) return;
      const oldWidthScale = view.widthScale, oldWidth = view.width, oldHeight = view.stageHeight;
      const ratio = Math.min(1, stage.clientWidth / image.naturalWidth, stage.clientHeight / image.naturalHeight);
      view.width = image.naturalWidth * ratio; view.height = image.naturalHeight * ratio;
      view.widthScale = Math.min(stage.clientWidth, 1000) / view.width;
      view.maxScale = Math.max(8, view.widthScale * 4);
      view.stageHeight = stage.clientHeight;
      image.style.width = view.width + 'px'; image.style.height = view.height + 'px';
      view.ready = true;
      if (initial) mode(image.naturalHeight / image.naturalWidth >= 2.5 && view.height * view.widthScale > stage.clientHeight * 1.5);
      else {
        if (view.reading) {
          const factor = view.width * view.widthScale / (oldWidth * oldWidthScale);
          view.scale *= view.widthScale / oldWidthScale; view.x *= factor;
          view.y = (oldHeight / 2 + view.y) * factor - stage.clientHeight / 2;
        }
        paint();
      }
    }
    function zoom(value, point = { x: 0, y: 0 }) {
      if (!view.ready) return;
      const next = clamp(value, 1, view.maxScale), ratio = next / view.scale;
      view.x = point.x - (point.x - view.x) * ratio;
      view.y = point.y - (point.y - view.y) * ratio;
      view.scale = next; paint();
    }
    function mode(reading) {
      view.reading = reading; view.scale = reading ? view.widthScale : 1; view.x = 0;
      view.y = reading ? Math.max(0, (view.height * view.scale - stage.clientHeight) / 2) : 0; paint();
    }
    return { view, paint, fit, zoom, mode, reset() { mode(false); } };
  }
  function gestures(stage, model) {
    const pointers = new Map();
    let start, lastTap, lastDouble = 0;
    function point(event) {
      const bounds = stage.getBoundingClientRect();
      return { x: event.clientX - bounds.left - bounds.width / 2, y: event.clientY - bounds.top - bounds.height / 2 };
    }
    function snapshot() {
      const points = [...pointers.values()], a = points[0], b = points[1];
      if (!a) { start = null; return; }
      start = { ...model.view, time: Date.now(), moved: false, multi: !!b,
        center: b ? { x: (a.x + b.x) / 2, y: (a.y + b.y) / 2 } : a,
        distance: b ? Math.hypot(a.x - b.x, a.y - b.y) : 0 };
    }
    function toggle(at) { const base = model.view.reading ? model.view.widthScale : 1; model.zoom(model.view.scale > base * 1.1 ? base : base * 2.5, at); lastDouble = Date.now(); lastTap = null; }
    stage.onpointerdown = event => {
      if (!model.view.ready || (event.pointerType === 'mouse' && event.button !== 0)) return;
      pointers.set(event.pointerId, point(event)); snapshot();
      if (pointers.size > 1) lastTap = null;
      try { stage.setPointerCapture(event.pointerId); } catch {}
    };
    stage.onpointermove = event => {
      if (!pointers.has(event.pointerId) || !start) return;
      pointers.set(event.pointerId, point(event));
      const [a, b] = [...pointers.values()];
      const center = b ? { x: (a.x + b.x) / 2, y: (a.y + b.y) / 2 } : a;
      if (b && start.distance > 0) {
        model.view.scale = clamp(start.scale * Math.hypot(a.x - b.x, a.y - b.y) / start.distance, 1, model.view.maxScale);
        const ratio = model.view.scale / start.scale;
        model.view.x = center.x - (start.center.x - start.x) * ratio;
        model.view.y = center.y - (start.center.y - start.y) * ratio;
      } else {
        model.view.x = start.x + center.x - start.center.x;
        model.view.y = start.y + center.y - start.center.y;
      }
      if (Math.hypot(center.x - start.center.x, center.y - start.center.y) > 8 || b) start.moved = true;
      model.paint();
    };
    function end(event) {
      if (!pointers.has(event.pointerId)) return;
      const at = point(event), tap = event.type === 'pointerup' && start && !start.multi && !start.moved && Date.now() - start.time < 280;
      pointers.delete(event.pointerId);
      if (tap) {
        if (lastTap && Date.now() - lastTap.time < 320 && Math.hypot(at.x - lastTap.x, at.y - lastTap.y) < 30) toggle(at);
        else lastTap = { ...at, time: Date.now() };
      }
      const multi = start?.multi; snapshot(); if (start && multi) start.moved = true;
    }
    stage.onpointerup = stage.onpointercancel = stage.onlostpointercapture = end;
    stage.ondblclick = event => { event.preventDefault(); if (Date.now() - lastDouble > 400) toggle(point(event)); };
    stage.addEventListener('wheel', event => {
      event.preventDefault();
      const unit = event.deltaMode === 1 ? 16 : event.deltaMode === 2 ? stage.clientHeight : 1;
      if (model.view.reading && !event.ctrlKey && !event.metaKey) {
        model.view.x -= event.deltaX * unit; model.view.y -= event.deltaY * unit; model.paint();
      } else model.zoom(model.view.scale * Math.exp(-clamp(event.deltaY * unit, -400, 400) * .002), point(event));
    }, { passive: false });
    stage.oncontextmenu = event => event.preventDefault();
    stage.ondragstart = event => event.preventDefault();
  }
  function open(item, url, trigger, releaseBlob) {
    if (current || returning || !trigger?.isConnected) return;
    let source; try { source = new URL(url, location.href); } catch { return; }
    const ownedBlob = releaseBlob && source.protocol === 'blob:' && source.origin === location.origin;
    if ((!ownedBlob && !['http:', 'https:'].includes(source.protocol)) || source.username || source.password) return;
    const name = item.display_name || item.file_name || '图片';
    const dialog = element('dialog', 'chat-image-viewer'); dialog.setAttribute('aria-label', '图片预览');
    const toolbar = element('header', 'chat-image-toolbar'), title = element('span', 'chat-image-title', name);
    const stage = element('div', 'chat-image-stage'), image = element('img', 'chat-image-original');
    const status = element('p', 'chat-image-status'), controls = element('footer', 'chat-image-controls');
    const zoomValue = element('span', 'chat-image-zoom', '100%'); zoomValue.setAttribute('aria-label', '缩放比例');
    let disposed = false, timer, ownsHistory = false;
    const token = 'image-' + Date.now() + '-' + Math.random().toString(36).slice(2);
    const chat = trigger.closest('#chatList'), scrollTop = chat?.scrollTop;
    const overflow = document.body.style.overflow;
    const minus = button('缩小图片', '−', () => model.zoom(model.view.scale / 1.5));
    const plus = button('放大图片', '+', () => model.zoom(model.view.scale * 1.5));
    const reset = button('恢复适合屏幕', '适合屏幕', () => model.reset());
    const read = button('长图阅读', '长图阅读', () => model.mode(true));
    const top = button('回到图片顶部', '↑', () => { model.view.y = model.view.height * model.view.scale; model.paint(); });
    const close = button('关闭图片预览', '×', () => finish());
    const retry = button('重试加载图片', '重新加载', () => load()); retry.hidden = true;
    const download = element('a', 'chat-image-download', '下载原图'); download.href = source.href;
    download.download = name; download.target = '_blank'; download.rel = 'noopener noreferrer';
    const model = transform(stage, image, view => {
      zoomValue.textContent = Math.round(view.scale * view.width / (image.naturalWidth || 1) * 100) + '%';
      minus.disabled = !view.ready || view.scale <= 1;
      reset.disabled = !view.ready || (!view.reading && view.scale <= 1);
      read.disabled = top.disabled = !view.ready;
      read.setAttribute('aria-pressed', String(view.reading));
      plus.disabled = !view.ready || view.scale >= view.maxScale;
    });
    function fail() {
      if (disposed) return;
      clearTimeout(timer); status.textContent = '图片加载失败，请重试或下载原图'; retry.hidden = false;
      image.hidden = true; model.view.ready = false; model.paint();
    }
    function load() {
      clearTimeout(timer); model.view.ready = false; model.reset(); image.hidden = true; retry.hidden = true;
      status.textContent = '正在加载图片…'; image.removeAttribute('src'); image.src = source.href;
      timer = setTimeout(fail, 15000);
    }
    function finish(fromHistory = false) {
      if (disposed) return;
      disposed = true; clearTimeout(timer); observer.disconnect(); stageObserver.disconnect(); root.removeEventListener('resize', resize);
      image.removeAttribute('src'); dialog.remove(); releaseBlob?.(); document.body.style.overflow = overflow; current = null;
      if (chat?.isConnected && trigger.isConnected) chat.scrollTop = scrollTop;
      if (trigger.isConnected) trigger.focus({ preventScroll: true });
      if (!fromHistory && ownsHistory && history.state?.[historyKey] === token) { returning = true; history.back(); }
    }
    function resize() { if (!disposed) model.fit(); }
    const stageObserver = new ResizeObserver(() => { if (!disposed && model.view.ready) model.fit(); });
    const observer = new MutationObserver(() => { if (!trigger.isConnected) finish(); });
    image.alt = name; image.draggable = false; image.referrerPolicy = 'no-referrer';
    image.onload = () => { if (!disposed) { clearTimeout(timer); image.hidden = false; status.textContent = ''; retry.hidden = true; model.fit(true); } };
    image.onerror = fail; status.setAttribute('role', 'status');
    dialog.oncancel = event => { event.preventDefault(); finish(); };
    dialog.onclose = () => finish();
    dialog.onkeydown = event => {
      if (['+', '=', '-', '0'].includes(event.key)) {
        event.preventDefault(); if (event.key === '0') model.reset(); else model.zoom(model.view.scale * (event.key === '-' ? 1 / 1.5 : 1.5));
      } else if (['Home', 'End', 'PageUp', 'PageDown'].includes(event.key)) {
        event.preventDefault();
        if (event.key === 'Home' || event.key === 'End') model.view.y = (event.key === 'Home' ? 1 : -1) * model.view.height * model.view.scale;
        else model.view.y += stage.clientHeight * .8 * (event.key === 'PageUp' ? 1 : -1);
        model.paint();
      } else if (model.view.scale > 1 && ['ArrowLeft', 'ArrowRight', 'ArrowUp', 'ArrowDown'].includes(event.key)) {
        event.preventDefault(); model.view.x += event.key === 'ArrowLeft' ? 40 : event.key === 'ArrowRight' ? -40 : 0;
        model.view.y += event.key === 'ArrowUp' ? 40 : event.key === 'ArrowDown' ? -40 : 0; model.paint();
      }
    };
    toolbar.append(close, title, download); stage.append(image); controls.append(minus, zoomValue, plus, reset, read, top);
    dialog.append(toolbar, stage, status, retry, controls);
    if (document.activeElement?.matches('input,textarea,[contenteditable="true"]')) document.activeElement.blur();
    document.body.append(dialog); document.body.style.overflow = 'hidden'; dialog.showModal(); close.focus({ preventScroll: true });
    current = { token, finish }; gestures(stage, model); observer.observe(document.body, { childList: true, subtree: true });
    root.addEventListener('resize', resize);
    stageObserver.observe(stage);
    try { history.pushState({ ...history.state, [historyKey]: token }, '', location.href); ownsHistory = true; } catch {}
    load();
    return () => finish();
  }
  function bind(image, item, url) {
    const trigger = button('查看大图：' + (item.display_name || item.file_name || '图片'), '', () => { if (Date.now() >= suppress) open(item, url, trigger); });
    let suppress = 0;
    trigger.className = 'chat-image-trigger'; image.tabIndex = -1;
    image.before(trigger); trigger.append(image);
    trigger.oncontextmenu = event => { suppress = Date.now() + 700; if (event.target === trigger) image.oncontextmenu?.(event); };
    trigger.onkeydown = event => { if (event.key === 'ContextMenu' || (event.key === 'F10' && event.shiftKey)) image.oncontextmenu?.(event); };
  }
  root.addEventListener('popstate', event => {
    returning = false;
    if (current && event.state?.[historyKey] !== current.token) current.finish(true);
  });
  root.addEventListener('pagehide', () => current?.finish(true));
  function openBlob(blob, item, trigger) {
    if (!(blob instanceof Blob) || !blob.type.startsWith('image/')) throw Error('附件类型不匹配');
    const url = URL.createObjectURL(blob), release = () => URL.revokeObjectURL(url);
    try {
      const close = open(item, url, trigger, release);
      if (!close) release();
      return close || (() => {});
    } catch (error) { release(); throw error; }
  }
  root.ElonSocialImageViewer = { bind, openBlob, close: () => current?.finish() };
})(globalThis);

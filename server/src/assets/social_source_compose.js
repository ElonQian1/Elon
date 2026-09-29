(function (root) {
  'use strict';
  const MAX_BYTES = 12 * 1024 * 1024, MAX_SECONDS = 120;
  let active = null, picker = null;
  function button(text, action) {
    const b = document.createElement('button'); b.type = 'button'; b.textContent = text;
    b.style.cssText = 'min-height:48px;padding:8px 18px;border:1px solid var(--line);border-radius:12px;background:var(--panel-2);color:var(--ink);margin:4px';
    b.onclick = action; return b;
  }
  function panel(label) {
    const dialog = document.createElement('dialog'); dialog.setAttribute('aria-label', label);
    dialog.className = 'social-compose-dialog';
    const style = document.createElement('style'); style.textContent = '.social-compose-dialog button:disabled{opacity:.45;cursor:default}'; dialog.append(style);
    dialog.style.cssText = 'box-sizing:border-box;background:var(--panel,#fff);color:var(--ink,#222);border:1px solid var(--line,#777);border-radius:16px;width:min(94vw,560px);max-height:86dvh;padding:18px;padding-bottom:max(18px,env(safe-area-inset-bottom));overflow:auto';
    const heading = document.createElement('h3'); heading.textContent = label; heading.tabIndex = -1; heading.autofocus = true;
    dialog.append(heading); return dialog;
  }
  function mount(dialog, options, cleanup) {
    active = dialog;
    const ownerTimer = setInterval(() => { if (options.identity !== options.owner()) dismiss(); }, 500);
    function dismiss() { cleanup(); if (dialog.open) dialog.close(); }
    const hide = () => dismiss();
    root.addEventListener('pagehide', hide);
    dialog.addEventListener('close', () => {
      clearInterval(ownerTimer); root.removeEventListener('pagehide', hide); cleanup();
      dialog.remove(); if (active === dialog) active = null;
    }, { once: true });
    document.body.append(dialog); dialog.showModal();
  }
  function capture(options) { return { ...options, contact: { ...options.contact }, identity: options.owner() }; }
  function open(options) {
    if (active) return;
    const snapshot = capture(options), mode = options.mode || 'media';
    picker?.remove();
    const input = document.createElement('input'); input.type = 'file'; input.multiple = mode !== 'camera';
    input.accept = ({ camera: 'image/*', media: 'image/*,video/*', audio: 'audio/*' })[mode] || '';
    if (mode === 'camera') input.setAttribute('capture', 'environment');
    input.tabIndex = -1; input.setAttribute('aria-hidden', 'true');
    input.style.cssText = 'position:fixed;width:1px;height:1px;opacity:0;left:0;bottom:0;pointer-events:none';
    const remove = () => { input.remove(); if (picker === input) picker = null; };
    input.addEventListener('cancel', remove, { once: true });
    input.addEventListener('change', () => {
      const files = Array.from(input.files || []); remove();
      if (files.length && snapshot.identity === snapshot.owner()) preview(snapshot, files);
    }, { once: true });
    // Keep the picker attached and activate it synchronously inside the user's tap.
    picker = input; document.body.append(input); input.click();
  }
  function preview(options, files) {
    if (active || options.identity !== options.owner()) return;
    const dialog = panel('分享预览'), notice = document.createElement('p'); notice.setAttribute('role', 'status');
    const target = document.createElement('p'); target.textContent = '发送给 ' + (options.contact.name || options.contact.nickname || options.contact.account || '当前会话');
    const text = document.createElement('textarea'); text.placeholder = '添加文字或文章链接'; text.setAttribute('aria-label', '附件说明'); text.maxLength = 4000;
    text.style.cssText = 'width:100%;box-sizing:border-box;min-height:88px;padding:12px;background:var(--panel-2);color:var(--ink);border:1px solid var(--line);border-radius:8px;font:inherit';
    const items = [], urls = []; let busy = false, uncertain = false, disposed = false, upload = null;
    const current = () => !disposed && dialog.isConnected && options.identity === options.owner();
    const cleanup = () => { disposed = true; upload?.abort(); urls.splice(0).forEach(url => URL.revokeObjectURL(url)); dialog.querySelectorAll('audio,video').forEach(media => media.pause()); };
    const cancel = button('取消', () => { cleanup(); dialog.close(); }), send = button('发送', submit);
    send.style.background = 'var(--brand)'; send.style.color = 'var(--brand-ink)';
    dialog.append(target, text, notice, cancel, send); mount(dialog, options, cleanup);
    if (files.length > 6 || files.some(file => file.size > MAX_BYTES || !file.size)) {
      notice.textContent = '每次最多 6 个附件，单个附件不超过 12 MB 且不能为空'; send.disabled = true; return;
    }
    notice.textContent = '请核对附件后发送';
    files.forEach(file => {
      const item = { file, source: null, attachment: null }; items.push(item);
      const name = document.createElement('p'); name.textContent = file.name + ' · ' + Math.max(1, Math.ceil(file.size / 1024)) + ' KB'; dialog.insertBefore(name, notice);
      const mime = file.type || '';
      const tag = mime.startsWith('image/') ? 'img' : mime.startsWith('audio/') ? 'audio' : mime.startsWith('video/') ? 'video' : '';
      if (tag) {
        const media = document.createElement(tag); media.src = URL.createObjectURL(file); urls.push(media.src);
        media.style.cssText = 'display:block;max-width:100%;max-height:200px;border-radius:8px';
        if (tag === 'img') media.alt = '待发送：' + file.name;
        else { media.controls = true; media.preload = 'metadata'; media.setAttribute('aria-label', '试听或预览：' + file.name); if (tag === 'video') media.playsInline = true; }
        dialog.insertBefore(media, notice);
      }
      // QR enrichment is optional: decoding must never prevent sending an image.
      if (mime.startsWith('image/') && root.ElonSourceLinks?.scan) {
        root.ElonSourceLinks.scan(file).then(links => {
          if (!current() || busy || !links.length) return;
          const select = document.createElement('select'); select.setAttribute('aria-label', '选择图片原文链接'); select.style.cssText = 'width:100%;min-height:48px;font:inherit';
          select.add(new Option('不附加链接', '')); links.forEach(link => select.add(new Option(link.url, link.url)));
          if (links.length === 1) { item.source = links[0]; select.value = links[0].url; }
          select.onchange = () => { item.source = links.find(link => link.url === select.value) || null; }; dialog.insertBefore(select, notice);
        }).catch(() => {});
      }
    });
    async function submit() {
      if (busy || uncertain || send.disabled || !current()) return;
      busy = true; send.disabled = true; text.disabled = true;
      dialog.querySelectorAll('select').forEach(select => { select.disabled = true; });
      try {
        for (let index = 0; index < items.length; index++) {
          const item = items[index];
          if (!current()) return;
          if (item.attachment) continue;
          const f = item.file, mime = f.type || 'application/octet-stream';
          const kind = options.voice ? 'voice' : mime.startsWith('image/') ? 'image' : mime.startsWith('audio/') ? 'audio' : mime.startsWith('video/') ? 'video' : 'file';
          const params = new URLSearchParams({ file_name: f.name, display_name: f.name, mime_type: mime, kind, conversation_id: `${options.kind}-${options.contact.id}` });
          notice.textContent = `正在上传 ${index + 1}/${items.length}：${f.name}`;
          upload = new AbortController(); const timer = setTimeout(() => upload?.abort(), 60000);
          try {
            const response = await options.api(`/api/user/${encodeURIComponent(options.userId)}/chat-attachments?${params}`, {
              method: 'POST', body: f, headers: { 'Content-Type': mime }, signal: upload.signal,
            });
            if (!response.ok) throw new Error(response.status === 413 ? '附件超过服务器大小限制' : '附件上传失败，请重试');
            const ref = (await response.json()).attachment;
            if (!ref?.url) throw new Error('附件上传响应异常');
            item.attachment = { ...ref, kind, mime_type: mime, ...(options.duration ? { duration_seconds: options.duration } : {}) };
          } finally { clearTimeout(timer); upload = null; }
        }
        if (!current()) return;
        uncertain = true; cancel.textContent = '关闭'; notice.textContent = '正在发送…';
        await options.send(options.kind, options.contact, text.value.trim(), items.map(item => ({ ...item.attachment, ...(item.source ? { source_link: item.source } : {}) })));
        if (current()) dialog.close();
      } catch (error) {
        if (!current()) return;
        notice.textContent = uncertain ? '发送结果未确认，请到聊天核对，避免重复发送。' : error.name === 'AbortError' ? '上传超时，请重试' : error.message;
        send.disabled = uncertain; text.disabled = uncertain;
        dialog.querySelectorAll('select').forEach(select => { select.disabled = uncertain; });
      } finally { busy = false; }
    }
  }
  function openVoice(rawOptions) {
    if (active) return;
    const options = capture(rawOptions), dialog = panel('录制语音'), status = document.createElement('p'); status.setAttribute('role', 'status');
    const player = document.createElement('audio'); player.controls = true; player.hidden = true; player.setAttribute('aria-label', '试听录音'); player.style.maxWidth = '100%';
    let stream, recorder, chunks = [], file = null, url = '', clock, permissionTimer, started = 0, seconds = 0, bytes = 0, generation = 0, closed = false;
    const tracksOff = () => { stream?.getTracks().forEach(track => track.stop()); stream = null; };
    const cleanup = () => {
      closed = true; generation++; clearInterval(clock); clearTimeout(permissionTimer);
      if (recorder?.state === 'recording') recorder.stop(); tracksOff(); player.pause();
      if (url) { URL.revokeObjectURL(url); url = ''; }
    };
    const valid = ticket => !closed && generation === ticket && options.identity === options.owner();
    function resetButtons() { start.disabled = false; start.textContent = file ? '重新录音' : '开始录音'; stop.disabled = true; review.disabled = !file; }
    function stopRecording() { if (recorder?.state === 'recording') { seconds = Math.max(1, Math.min(MAX_SECONDS, Math.ceil((Date.now() - started) / 1000))); recorder.stop(); tracksOff(); clearInterval(clock); stop.disabled = true; } }
    const start = button('开始录音', begin), stop = button('停止录音', stopRecording), review = button('使用录音', () => {
      if (!file || options.identity !== options.owner()) return;
      const recorded = file, duration = seconds; cleanup(); dialog.close(); dialog.remove(); if (active === dialog) active = null;
      preview({ ...options, voice: true, duration }, [recorded]);
    });
    const choose = button('选择音频文件', () => {
      cleanup(); dialog.close(); dialog.remove(); if (active === dialog) active = null;
      if (options.identity === options.owner()) open({ ...options, mode: 'audio' });
    });
    const cancel = button('取消', () => { cleanup(); dialog.close(); });
    dialog.append(status, player, start, stop, review, choose, cancel); mount(dialog, options, cleanup); resetButtons();
    const supported = root.isSecureContext && navigator.mediaDevices?.getUserMedia && root.MediaRecorder;
    status.textContent = supported ? '最长录制 2 分钟，停止后可试听，确认后再发送。' : !root.isSecureContext ? '录音需要 HTTPS 页面；也可以选择已有音频文件。' : '当前浏览器无法录音，可以选择已有音频文件。';
    start.disabled = !supported;
    async function begin() {
      if (start.disabled || closed) return;
      const ticket = ++generation; file = null; bytes = 0; chunks = []; player.pause(); player.hidden = true;
      if (url) { URL.revokeObjectURL(url); url = ''; }
      start.disabled = true; review.disabled = true; status.textContent = '请允许使用麦克风…';
      permissionTimer = setTimeout(() => { if (valid(ticket)) { generation++; status.textContent = '尚未取得麦克风权限，请重试或选择音频文件。'; resetButtons(); } }, 20000);
      try {
        const acquired = await navigator.mediaDevices.getUserMedia({ audio: true });
        if (!valid(ticket)) { acquired.getTracks().forEach(track => track.stop()); return; }
        clearTimeout(permissionTimer); stream = acquired;
        const mime = ['audio/mp4;codecs=mp4a.40.2', 'audio/webm;codecs=opus', 'audio/webm', 'audio/mp4', 'audio/ogg;codecs=opus'].find(type => root.MediaRecorder.isTypeSupported?.(type));
        recorder = new root.MediaRecorder(stream, mime ? { mimeType: mime } : undefined);
        const recording = recorder;
        recording.ondataavailable = event => {
          if (!valid(ticket) || !event.data.size) return;
          bytes += event.data.size; chunks.push(event.data); if (bytes > MAX_BYTES) stopRecording();
        };
        recording.onerror = () => { if (valid(ticket)) { generation++; tracksOff(); clearInterval(clock); status.textContent = '录音中断，请重新录音或选择音频文件。'; resetButtons(); } };
        recording.onstop = () => {
          if (!valid(ticket)) return; tracksOff(); clearInterval(clock);
          seconds = Math.max(1, Math.min(MAX_SECONDS, Math.ceil((Date.now() - started) / 1000)));
          if (!bytes || bytes > MAX_BYTES) { status.textContent = bytes ? '录音超过 12 MB，请缩短后重录。' : '没有录到声音数据，请重试。'; resetButtons(); return; }
          const type = recording.mimeType || chunks[0]?.type || mime;
          if (!type) { status.textContent = '录音格式不可用，请选择音频文件。'; resetButtons(); return; }
          const extension = /mp4/i.test(type) ? 'm4a' : /ogg/i.test(type) ? 'ogg' : 'webm';
          file = new File(chunks, `语音-${Date.now()}.${extension}`, { type }); chunks = [];
          url = URL.createObjectURL(file); player.src = url; player.hidden = false;
          status.textContent = `已录制 ${seconds} 秒，可试听或重新录音。`; resetButtons();
        };
        started = Date.now(); seconds = 0; recorder.start(1000); stop.disabled = false;
        status.textContent = '正在录音 0 秒 / 120 秒';
        clock = setInterval(() => {
          const elapsed = Math.floor((Date.now() - started) / 1000);
          status.textContent = `正在录音 ${elapsed} 秒 / 120 秒`; if (elapsed >= MAX_SECONDS) stopRecording();
        }, 250);
      } catch (error) {
        if (!valid(ticket)) return; clearTimeout(permissionTimer); tracksOff();
        status.textContent = error.name === 'NotAllowedError' ? '麦克风权限未开启，请在浏览器设置中允许，或选择音频文件。' : '无法启动麦克风，请重试或选择音频文件。'; resetButtons();
      }
    }
    const visibility = () => { if (document.hidden) { if (recorder?.state === 'recording') stopRecording(); else if (start.disabled && !closed) { generation++; clearTimeout(permissionTimer); resetButtons(); } } };
    document.addEventListener('visibilitychange', visibility);
    dialog.addEventListener('close', () => document.removeEventListener('visibilitychange', visibility), { once: true });
  }
  root.ElonSourceCompose = { open, openVoice };
})(globalThis);

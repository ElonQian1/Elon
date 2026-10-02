/* Shared PC/PWA attachment surface. Only visible media loads; playback is always explicit. */
(() => {
  function mount(host, row, options) {
    let active = true, loading = false, url = '', media, observer, requestedPlay = false, closeImage;
    const valid = () => active && options.current();
    const make = (tag, text) => { const n = document.createElement(tag); if (text != null) n.textContent = text; return n; };
    const action = (text, fn) => { const n = make('button', text); n.type = 'button'; n.onclick = fn; return n; };
    const status = make('small'); status.setAttribute('role', 'status');
    const frame = make('div'); frame.className = 'chat-record-media-frame';
    const caption = make('small', row.filename); caption.className = 'chat-record-filename';
    host.append(frame, caption, status);
    const play = action('▶', () => { if (!url) { void load(true); return; } showVideo(); });
    play.className = 'chat-record-video-play'; play.setAttribute('aria-label', '播放视频');
    function showVideo() {
      media = make('video'); media.src = url; media.controls = true; media.preload = 'metadata'; media.playsInline = true; media.setAttribute('aria-label', row.filename || '视频');
      media.onerror = () => { status.textContent = '此视频格式暂不支持播放'; };
      frame.replaceChildren(media); media.play().catch(() => { status.textContent = '点击播放器继续播放'; });
    }
    async function load(autoplay = false) {
      requestedPlay ||= autoplay;
      if (loading || !valid()) return;
      loading = true; status.textContent = '正在读取附件…';
      try {
        const blob = await options.load(); if (!valid()) return;
        if ((row.kind === 'image' && !blob.type.startsWith('image/')) || (row.kind === 'video' && !blob.type.startsWith('video/'))) throw Error('附件类型不匹配');
        if (url) URL.revokeObjectURL(url); url = URL.createObjectURL(blob);
        status.textContent = ''; frame.replaceChildren();
        if (row.kind === 'image') {
          const name = row.filename || '图片', img = make('img'); img.src = url; img.alt = name;
          const preview = action('', () => {
            if (!valid()) return;
            try {
              if (!options.openImage) throw Error('图片预览暂不可用，请重新打开聊天记录');
              closeImage?.(); closeImage = options.openImage(blob, name, preview);
            } catch { status.textContent = '图片预览暂不可用，请重新打开聊天记录'; }
          });
          preview.className = 'chat-record-image-trigger'; preview.setAttribute('aria-label', '查看大图：' + name);
          preview.append(img); frame.append(preview);
        } else if (row.kind === 'video') {
          frame.classList.add('chat-record-video-frame'); frame.append(play);
          if (requestedPlay) { showVideo(); return; }
          try {
            const poster = await ElonRecordVideo.poster(options.scope, blob); if (!valid() || media) return;
            const img = make('img'); img.alt = ''; img.src = poster.poster; frame.prepend(img);
            frame.style.aspectRatio = String(Math.max(.55, Math.min(16 / 9, poster.width / poster.height)));
            const duration = ElonRecordPresentation.duration(poster.seconds);
            if (duration) { const time = make('span', duration); time.className = 'chat-record-duration'; frame.append(time); }
          } catch { if (valid() && !media) status.textContent = '缩略图暂不可用，仍可播放'; }
        } else { const a = make('a', '下载附件'); a.href = url; a.download = row.filename || '附件'; frame.append(a); }
      } catch (e) { if (valid()) { status.textContent = e.message || '附件读取失败'; frame.replaceChildren(action('重试', () => load())); } }
      finally { loading = false; }
    }
    if (row.kind === 'video') { frame.classList.add('chat-record-video-frame'); frame.append(play); }
    if (['image', 'video'].includes(row.kind)) {
      observer = new IntersectionObserver(entries => { if (entries.some(e => e.isIntersecting)) { observer.disconnect(); void load(); } }); observer.observe(host);
    } else frame.append(action('读取附件', () => load()));
    return () => { active = false; observer?.disconnect(); closeImage?.(); media?.pause(); if (url) URL.revokeObjectURL(url); host.replaceChildren(); };
  }
  globalThis.ElonRecordMedia = { mount };
})();

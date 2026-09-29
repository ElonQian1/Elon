(function (root) {
  'use strict';
  const players = new Set();
  let active;
  const seconds = value => Number.isFinite(Number(value)) && Number(value) > 0 ? Math.ceil(Number(value)) : 0;
  function mount(box, item, url, download) {
    const player = document.createElement('audio'), button = document.createElement('button');
    const length = document.createElement('span'), status = document.createElement('span');
    const options = document.createElement('details'), summary = document.createElement('summary');
    let duration = seconds(item.duration_seconds), disposed = false, wanted = false, attempt = 0;
    player.preload = 'none'; player.hidden = true; player.src = url;
    player.setAttribute('aria-label', '语音音频');
    button.type = 'button'; button.className = 'voice-message-play';
    button.innerHTML = '<svg viewBox="0 0 24 24" aria-hidden="true"><path d="M5 9v6m4-8a7 7 0 0 1 0 10m4-13a11 11 0 0 1 0 16" fill="none" stroke="currentColor" stroke-width="2.4" stroke-linecap="round"/></svg>';
    button.append(length); status.className = 'voice-message-status'; status.setAttribute('role', 'status');
    options.className = 'voice-message-options'; summary.textContent = '···'; summary.setAttribute('aria-label', '语音选项');
    download.textContent = '下载语音'; options.append(summary, download);
    box.classList.add('voice-message-player'); box.append(button, player, options, status);
    function state(value, message = '') {
      if (disposed) return;
      button.dataset.state = value; button.setAttribute('aria-pressed', String(value === 'playing'));
      const action = value === 'playing' ? '暂停语音' : value === 'loading' ? '取消加载' : value === 'error' ? '重试语音' : '播放语音';
      button.setAttribute('aria-label', action + (duration ? '，' + duration + '秒' : ''));
      length.textContent = value === 'loading' ? '加载中' : value === 'error' ? '重试' : duration ? duration + '″' : '语音';
      status.textContent = message; status.hidden = !message;
    }
    const control = {
      box,
      stop() { wanted = false; attempt++; player.pause(); if (active === control) active = null; state('ready'); },
      dispose() {
        if (disposed) return;
        control.stop(); disposed = true; player.removeAttribute('src'); player.load(); players.delete(control);
      },
    };
    function failed(error) {
      if (disposed || !wanted) return;
      wanted = false; if (active === control) active = null;
      if (error?.name === 'NotAllowedError') return state('ready', '请再次点击语音播放');
      state('error', '语音加载失败，点击重试或下载语音');
    }
    button.onclick = () => {
      if (wanted) { control.stop(); return; }
      active?.stop();
      document.querySelectorAll('audio').forEach(other => { if (other !== player) other.pause(); });
      if (player.error) { player.src = url; player.load(); }
      if (player.ended) player.currentTime = 0;
      wanted = true; active = control; const id = ++attempt; state('loading');
      // Invoke play inside the original tap, without an async fetch losing iOS activation.
      try { player.play()?.catch(error => { if (id === attempt) failed(error); }); } catch (error) { failed(error); }
    };
    player.onloadedmetadata = () => {
      duration = seconds(player.duration) || duration;
      if (!wanted) state('ready');
    };
    player.onplaying = () => { if (wanted) state('playing'); else player.pause(); };
    player.onwaiting = () => { if (wanted) state('loading'); };
    player.onpause = () => { if (!disposed && !player.error) { wanted = false; if (active === control) active = null; state('ready'); } };
    player.onended = () => control.stop();
    player.onerror = () => failed();
    players.add(control); state('ready');
    return control.dispose;
  }
  function stopWithin(node) { for (const player of players) if (node.contains(player.box)) player.stop(); }
  // Dialog sources and deleted/recalled messages can also own players.
  new MutationObserver(() => { for (const player of players) if (!player.box.isConnected) player.dispose(); })
    .observe(document.documentElement, { childList: true, subtree: true });
  document.addEventListener('visibilitychange', () => { if (document.hidden) active?.stop(); });
  root.addEventListener('pagehide', () => active?.stop());
  root.ElonSocialVoice = { mount, stopWithin };
})(globalThis);

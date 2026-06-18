(function () {
  const kit = window.ElonPcKit || {};
  const readToken = typeof kit.readToken === 'function'
    ? kit.readToken
    : function () {
      return localStorage.getItem('lodex_token') || localStorage.getItem('elon_token') || '';
    };

  const MAX_RECONNECT_MS = 30000;
  const DONE_EVENT_TYPE = 'project_task_done';

  let socket = null;
  let reconnectTimer = 0;
  let reconnectDelay = 1200;
  let connectedToken = '';
  let audioContext = null;
  let lastEventKey = '';
  let lastEventAt = 0;
  let doneCount = 0;
  let titleTimer = 0;
  const baseTitle = document.title;

  function makeWsUrl(token) {
    const url = new URL('/ws/app', location.href);
    url.protocol = location.protocol === 'https:' ? 'wss:' : 'ws:';
    url.searchParams.set('token', token);
    return url.toString();
  }

  function clearReconnectTimer() {
    if (reconnectTimer) {
      clearTimeout(reconnectTimer);
      reconnectTimer = 0;
    }
  }

  function closeSocket() {
    if (socket) {
      socket.onopen = null;
      socket.onmessage = null;
      socket.onclose = null;
      socket.onerror = null;
      socket.close();
      socket = null;
    }
  }

  function scheduleReconnect() {
    clearReconnectTimer();
    reconnectTimer = setTimeout(connect, reconnectDelay);
    reconnectDelay = Math.min(MAX_RECONNECT_MS, Math.round(reconnectDelay * 1.8));
  }

  function connect() {
    const token = readToken();
    clearReconnectTimer();

    if (!token) {
      connectedToken = '';
      closeSocket();
      return;
    }
    if (socket && connectedToken === token && socket.readyState <= WebSocket.OPEN) {
      return;
    }

    closeSocket();
    connectedToken = token;
    socket = new WebSocket(makeWsUrl(token));
    socket.onopen = function () {
      reconnectDelay = 1200;
    };
    socket.onmessage = function (event) {
      handleMessage(event.data);
    };
    socket.onclose = function () {
      socket = null;
      if (readToken()) scheduleReconnect();
    };
    socket.onerror = function () {
      if (socket) socket.close();
    };
  }

  function handleMessage(raw) {
    let data;
    try {
      data = JSON.parse(raw);
    } catch (_) {
      return;
    }
    if (!data || data.type !== DONE_EVENT_TYPE) return;

    const key = [
      data.projectId || '',
      data.conversationId || '',
      data.message || ''
    ].join('|');
    const now = Date.now();
    if (key === lastEventKey && now - lastEventAt < 60000) return;
    lastEventKey = key;
    lastEventAt = now;

    window.dispatchEvent(new CustomEvent('elon:project-task-done', { detail: data }));
    notifyTaskDone(data);
  }

  function notifyTaskDone(data) {
    showBrowserNotification(data);
    playDoneSound();
    markTitle();
  }

  function showBrowserNotification(data) {
    if (!('Notification' in window) || Notification.permission !== 'granted') return;

    const message = String(data.message || '项目会话已完成').trim();
    const body = data.apkUrl
      ? message + '\nAPK 可以下载测试。'
      : message;
    try {
      const notification = new Notification('一龙会话已完成', {
        body: body.slice(0, 220),
        tag: 'elon-project-task-done-' + (data.conversationId || data.projectId || Date.now()),
        renotify: true
      });
      notification.onclick = function () {
        window.focus();
        notification.close();
      };
    } catch (_) {}
  }

  function markTitle() {
    doneCount += 1;
    document.title = '(' + doneCount + ') 会话已完成 - ' + baseTitle;
    if (titleTimer) clearTimeout(titleTimer);
    titleTimer = setTimeout(function () {
      doneCount = 0;
      document.title = baseTitle;
    }, 20000);
  }

  function ensureAudioContext() {
    const AudioContextClass = window.AudioContext || window.webkitAudioContext;
    if (!AudioContextClass) return null;
    if (!audioContext) audioContext = new AudioContextClass();
    return audioContext;
  }

  function playDoneSound() {
    const ctx = ensureAudioContext();
    if (!ctx) return;
    const play = function () {
      const gain = ctx.createGain();
      gain.gain.setValueAtTime(0.0001, ctx.currentTime);
      gain.gain.exponentialRampToValueAtTime(0.18, ctx.currentTime + 0.02);
      gain.gain.exponentialRampToValueAtTime(0.0001, ctx.currentTime + 0.42);
      gain.connect(ctx.destination);
      [740, 988].forEach(function (frequency, index) {
        const osc = ctx.createOscillator();
        const start = ctx.currentTime + index * 0.16;
        osc.type = 'sine';
        osc.frequency.setValueAtTime(frequency, start);
        osc.connect(gain);
        osc.start(start);
        osc.stop(start + 0.2);
      });
    };
    if (ctx.state === 'suspended') {
      ctx.resume().then(play).catch(function () {});
    } else {
      play();
    }
  }

  function prepareUserGestureFeatures() {
    const ctx = ensureAudioContext();
    if (ctx && ctx.state === 'suspended') {
      ctx.resume().catch(function () {});
    }
    if ('Notification' in window && Notification.permission === 'default') {
      try {
        const request = Notification.requestPermission();
        if (request && typeof request.catch === 'function') request.catch(function () {});
      } catch (_) {}
    }
  }

  ['pointerdown', 'keydown'].forEach(function (eventName) {
    window.addEventListener(eventName, prepareUserGestureFeatures, { once: true, passive: true });
  });
  window.addEventListener('storage', connect);
  setInterval(connect, 2500);
  connect();
})();

/* Posters are private derived cache entries, consulted only AFTER an authorized asset read. */
(() => {
  'use strict';
  const memory = new Map();
  let database;
  function db() {
    return database ||= new Promise(resolve => {
      try {
        const r = indexedDB.open('elon-record-posters-v1', 1);
        r.onupgradeneeded = () => r.result.createObjectStore('posters', { keyPath: 'key' });
        r.onsuccess = () => resolve(r.result); r.onerror = r.onblocked = () => resolve(null);
      } catch { resolve(null); }
    });
  }
  async function stored(key, value) {
    const database = await db(); if (!database) return null;
    return new Promise(resolve => {
      try {
        const tx = database.transaction('posters', value ? 'readwrite' : 'readonly'), store = tx.objectStore('posters');
        const r = value ? store.put({ key, ...value, at: Date.now() }) : store.get(key);
        r.onsuccess = () => resolve(value || r.result); r.onerror = () => resolve(null);
        tx.onerror = tx.onabort = () => resolve(null);
        if (value) {
          const all = store.getAll(); all.onsuccess = () => {
            const rows = all.result.sort((a, b) => b.at - a.at); let bytes = 0;
            rows.forEach((row, i) => { bytes += row.poster?.length || 0; if (i >= 64 || bytes > 16 * 1024 * 1024 || row.at < Date.now() - 7 * 86400000) store.delete(row.key); });
          };
        }
      } catch { resolve(null); }
    });
  }
  function extract(blob) {
    return new Promise((resolve, reject) => {
      const video = document.createElement('video'), url = URL.createObjectURL(blob);
      let timer;
      function finish(error, result) { clearTimeout(timer); video.onloadeddata = video.onerror = null; video.pause(); video.removeAttribute('src'); video.load(); URL.revokeObjectURL(url); error ? reject(error) : resolve(result); }
      video.muted = true; video.playsInline = true; video.preload = 'auto';
      video.onloadeddata = () => {
        try {
          const scale = Math.min(1, 480 / Math.max(video.videoWidth, video.videoHeight));
          const canvas = document.createElement('canvas'); canvas.width = Math.max(1, Math.round(video.videoWidth * scale)); canvas.height = Math.max(1, Math.round(video.videoHeight * scale));
          canvas.getContext('2d').drawImage(video, 0, 0, canvas.width, canvas.height);
          finish(null, { poster: canvas.toDataURL('image/jpeg', .75), seconds: Number.isFinite(video.duration) ? video.duration : 0, width: canvas.width, height: canvas.height });
        } catch (e) { finish(e); }
      };
      video.onerror = () => finish(Error('暂时无法生成缩略图，仍可尝试播放'));
      timer = setTimeout(() => finish(Error('缩略图读取超时，仍可尝试播放')), 10000);
      video.src = url;
    });
  }
  let queue = Promise.resolve();
  async function poster(scope, blob) {
    const key = `${scope}:${blob.size}:${blob.type}`;
    if (memory.has(key)) return memory.get(key);
    const task = queue.catch(() => {}).then(async () => {
      const cached = await stored(key);
      if (cached && cached.at > Date.now() - 7 * 86400000 && cached.poster?.startsWith('data:image/jpeg;')) return cached;
      const value = await extract(blob); await stored(key, value); return value;
    });
    queue = task.catch(() => {}); memory.set(key, task);
    while (memory.size > 32) memory.delete(memory.keys().next().value);
    task.catch(() => memory.delete(key)); return task;
  }
  globalThis.ElonRecordVideo = { poster };
})();

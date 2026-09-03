(function (root, factory) {
  'use strict';

  const api = factory(root);
  if (typeof module === 'object' && module.exports) {
    module.exports = Object.freeze(Object.assign({ create: factory }, api));
  }
  if (root && root.location && root.location.origin === 'https://chatgpt.com') {
    const current = root.__elonChatGptDictationRuntime;
    if (!current || Number(current.version) < Number(api.version)) {
      root.__elonChatGptDictationRuntime = Object.freeze(api);
    }
  }
})(typeof window === 'object' ? window : globalThis, function (root) {
  'use strict';

  const VERSION = 1;
  const POLL_MS = 80;
  const watchers = new Set();
  let patched = false;

  function schedule(callback, delayMs) {
    return (root && typeof root.setTimeout === 'function' ? root.setTimeout : setTimeout)(callback, delayMs);
  }

  function clearScheduled(timer) {
    return (root && typeof root.clearTimeout === 'function' ? root.clearTimeout : clearTimeout)(timer);
  }

  function audioRequested(constraints) {
    return !!(constraints && constraints.audio);
  }

  function patchGetUserMedia() {
    if (patched) return true;
    const mediaDevices = root && root.navigator && root.navigator.mediaDevices;
    const original = mediaDevices && typeof mediaDevices.getUserMedia === 'function'
      ? mediaDevices.getUserMedia.bind(mediaDevices)
      : null;
    if (!original) return false;
    try {
      mediaDevices.getUserMedia = function (constraints) {
        const markers = [];
        if (audioRequested(constraints)) {
          watchers.forEach((watcher) => {
            const marker = watcher.beforeRequest();
            if (marker) markers.push({ watcher, marker });
          });
        }
        return original(constraints).then((stream) => {
          markers.forEach(({ watcher, marker }) => watcher.accept(marker, stream));
          return stream;
        }, (error) => {
          markers.forEach(({ watcher, marker }) => watcher.reject(marker));
          throw error;
        });
      };
      patched = true;
      return true;
    } catch (_) {
      return false;
    }
  }

  function waitUntil(predicate, timeoutMs) {
    const deadline = Date.now() + Math.max(0, Number(timeoutMs) || 0);
    return new Promise((resolve) => {
      function inspect() {
        let matched = false;
        try { matched = predicate() === true; } catch (_) {}
        if (matched || Date.now() >= deadline) {
          resolve(matched);
          return;
        }
        schedule(inspect, POLL_MS);
      }
      inspect();
    });
  }

  function createCaptureTracker(options) {
    const config = options || {};
    const armWindowMs = Math.max(1000, Number(config.armWindowMs) || 12000);
    let generation = 0;
    let armedUntil = 0;
    let pending = 0;
    const tracks = new Map();

    function prune() {
      tracks.forEach((_token, track) => {
        if (!track || track.readyState === 'ended') tracks.delete(track);
      });
    }

    const watcher = {
      beforeRequest() {
        if (Date.now() > armedUntil) return 0;
        pending += 1;
        return generation;
      },
      accept(token, stream) {
        if (token === generation) pending = Math.max(0, pending - 1);
        if (token !== generation) return;
        const audioTracks = stream && typeof stream.getAudioTracks === 'function'
          ? stream.getAudioTracks()
          : [];
        audioTracks.forEach((track) => {
          if (!track || track.readyState === 'ended') return;
          tracks.set(track, token);
          if (typeof track.addEventListener === 'function') {
            track.addEventListener('ended', () => tracks.delete(track), { once: true });
          }
        });
      },
      reject(token) {
        if (token === generation) pending = Math.max(0, pending - 1);
      }
    };

    watchers.add(watcher);
    patchGetUserMedia();

    function arm() {
      generation += 1;
      pending = 0;
      armedUntil = Date.now() + armWindowMs;
      prune();
    }

    function release() {
      armedUntil = 0;
      pending = 0;
      prune();
    }

    function active() {
      prune();
      return Array.from(tracks.values()).some((token) => token === generation);
    }

    function pendingNow() {
      return pending > 0 && Date.now() <= armedUntil;
    }

    function stopTracked() {
      tracks.forEach((token, track) => {
        if (token !== generation || !track || typeof track.stop !== 'function') return;
        try { track.stop(); } catch (_) {}
      });
      release();
    }

    function dispose() {
      release();
      watchers.delete(watcher);
    }

    return Object.freeze({
      arm,
      finish: release,
      release,
      active,
      pending: pendingNow,
      stopTracked,
      dispose,
      waitForActive: (timeoutMs) => waitUntil(active, timeoutMs),
      waitForInactive: (timeoutMs) => waitUntil(() => !active() && !pendingNow(), timeoutMs)
    });
  }

  return Object.freeze({
    version: VERSION,
    createCaptureTracker,
    waitUntil,
    clearScheduled
  });
});

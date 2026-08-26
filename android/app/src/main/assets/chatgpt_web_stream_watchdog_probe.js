(function (root, factory) {
  'use strict';

  const api = factory();
  if (typeof module !== 'undefined' && module.exports) module.exports = api;
  if (root) root.__elonChatGptStreamWatchdogProbe = Object.freeze(api);
})(typeof window !== 'undefined' ? window : null, function () {
  'use strict';

  function create(options) {
    const now = typeof options.now === 'function' ? options.now : Date.now;
    const scheduleTimer = options.scheduleTimer;
    const cancelTimer = options.cancelTimer;
    const onResult = options.onResult;
    const timeoutMs = Math.max(5000, Number(options.timeoutMs) || 30000);
    const minimumStallMs = Math.max(1000, Number(options.minimumStallMs) || 3500);
    let requestId = '';
    let phase = 'idle';
    let firstPrivateAt = 0;
    let timeoutTimer = 0;

    function clearTimer() {
      if (timeoutTimer && typeof cancelTimer === 'function') cancelTimer(timeoutTimer);
      timeoutTimer = 0;
    }

    function reset() {
      clearTimer();
      requestId = '';
      phase = 'idle';
      firstPrivateAt = 0;
    }

    function finish(ok, detail) {
      const completedRequestId = requestId;
      reset();
      if (completedRequestId && typeof onResult === 'function') {
        onResult(completedRequestId, ok === true, String(detail || ''));
      }
    }

    function arm(nextRequestId, streaming) {
      if (phase !== 'idle') return { accepted: false, detail: 'watchdog_probe_already_active' };
      if (streaming === true) return { accepted: false, detail: 'stream_already_active' };
      const value = String(nextRequestId || '');
      if (!/^mcp_[a-z0-9]{1,32}$/.test(value)) {
        return { accepted: false, detail: 'invalid_request_id' };
      }
      requestId = value;
      phase = 'armed';
      if (typeof scheduleTimer === 'function') {
        timeoutTimer = scheduleTimer(timeoutMs, function () {
          if (phase !== 'idle') finish(false, 'private_stream_watchdog_timeout');
        });
      }
      return { accepted: true, detail: '' };
    }

    function observePrivateUpdate() {
      if (phase === 'armed') {
        phase = 'stalling';
        firstPrivateAt = now();
        return false;
      }
      return phase === 'stalling';
    }

    function watchdogFired(schedule) {
      if (phase !== 'stalling' || !schedule ||
          schedule.mode !== 'private_stream_watchdog') return false;
      if (now() - firstPrivateAt < minimumStallMs) return false;
      finish(true, 'private_stream_watchdog_fired');
      return true;
    }

    function dispose() {
      reset();
    }

    function state() {
      return Object.freeze({ active: phase !== 'idle', phase });
    }

    return Object.freeze({ arm, observePrivateUpdate, watchdogFired, dispose, state });
  }

  return Object.freeze({ create });
});

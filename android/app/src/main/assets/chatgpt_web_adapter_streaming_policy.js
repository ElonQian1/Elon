(function (root, factory) {
  'use strict';

  const policy = factory();
  if (typeof module !== 'undefined' && module.exports) module.exports = policy;
  if (root) root.__elonChatGptStreamingPolicy = Object.freeze(policy);
})(typeof window !== 'undefined' ? window : null, function () {
  'use strict';

  function create(options) {
    const clock = typeof options.now === 'function' ? options.now : Date.now;
    const settleMs = Math.max(500, Number(options.settleMs) || 2500);
    const waitTimeoutMs = Math.max(settleMs, Number(options.waitTimeoutMs) || 30000);
    const completionQuietMs = Math.max(200, Number(options.completionQuietMs) || 400);
    const heartbeatMs = Math.max(200, Number(options.heartbeatMs) || 400);
    const privateStreamWatchdogMs = Math.max(
      heartbeatMs,
      Number(options.privateStreamWatchdogMs) || 4000
    );
    const scheduleTimer = options.scheduleTimer;
    const cancelTimer = options.cancelTimer;
    let heartbeatTimer = 0;
    let active = false;
    let allowSameTurn = false;
    let baselineKey = '';
    let targetKey = '';
    let fingerprint = '';
    let startedAt = 0;
    let changedAt = 0;

    function idle() {
      return { active: false, assistantKey: '' };
    }

    function current() {
      return { active, assistantKey: targetKey };
    }

    function reset() {
      if (heartbeatTimer && typeof cancelTimer === 'function') cancelTimer(heartbeatTimer);
      heartbeatTimer = 0;
      active = false;
      allowSameTurn = false;
      baselineKey = '';
      targetKey = '';
      fingerprint = '';
      startedAt = 0;
      changedAt = 0;
      return idle();
    }

    function begin(observation, beginOptions) {
      if (active && targetKey) return current();
      const value = observation || {};
      const timestamp = clock();
      active = true;
      allowSameTurn = !!(beginOptions && beginOptions.allowSameTurn);
      baselineKey = String(value.key || '');
      targetKey = allowSameTurn ? baselineKey : '';
      fingerprint = targetKey ? String(value.fingerprint || '') : '';
      startedAt = timestamp;
      changedAt = timestamp;
      return current();
    }

    function observe(observation, officialActive) {
      const value = observation || {};
      const key = String(value.key || '');
      const nextFingerprint = String(value.fingerprint || '');
      const timestamp = clock();

      if (!active) {
        if (officialActive !== true && value.pending !== true) return idle();
        active = true;
        allowSameTurn = true;
        baselineKey = '';
        targetKey = key;
        fingerprint = nextFingerprint;
        startedAt = timestamp;
        changedAt = timestamp;
      }

      if (!targetKey) {
        if (key && (allowSameTurn || key !== baselineKey || value.pending === true)) {
          targetKey = key;
          fingerprint = nextFingerprint;
          changedAt = timestamp;
        } else if (officialActive !== true && timestamp - startedAt >= waitTimeoutMs) {
          return reset();
        }
        return current();
      }

      if (key && key !== targetKey) {
        targetKey = key;
        fingerprint = nextFingerprint;
        changedAt = timestamp;
      }
      const changed = nextFingerprint !== fingerprint;
      if (changed) {
        fingerprint = nextFingerprint;
        changedAt = timestamp;
      }
      if (officialActive === true || value.pending === true) return current();
      if (value.completionVisible === true && !changed &&
          timestamp - changedAt >= completionQuietMs) return reset();
      if (timestamp - changedAt >= settleMs) return reset();
      return current();
    }

    function scheduleNext(shouldSchedule, next, scheduleOptions) {
      if (heartbeatTimer && typeof cancelTimer === 'function') cancelTimer(heartbeatTimer);
      heartbeatTimer = 0;
      if (shouldSchedule !== true || typeof scheduleTimer !== 'function' || typeof next !== 'function') return;
      const delayMs = scheduleOptions && scheduleOptions.privateStreamObserved === true
        ? privateStreamWatchdogMs
        : heartbeatMs;
      heartbeatTimer = scheduleTimer(delayMs, function () {
        heartbeatTimer = 0;
        next();
      });
    }

    function dispose() {
      reset();
    }

    return Object.freeze({ begin, observe, reset, scheduleNext, dispose });
  }

  function officialActive(document, composer, visible) {
    const isVisible = typeof visible === 'function' ? visible : function (node) { return !!node; };
    const direct = document.querySelector(
      '[data-testid="stop-button"], button[data-testid*="stop" i], '
      + 'main [data-is-streaming="true"], main [data-streaming="true"], main .result-streaming'
    );
    if (isVisible(direct)) return true;
    const scope = composer && composer.closest('form');
    return !!scope && Array.from(scope.querySelectorAll('button')).some(function (button) {
      if (!isVisible(button)) return false;
      const label = [
        button.getAttribute('data-testid'),
        button.getAttribute('aria-label'),
        button.getAttribute('title'),
        button.textContent
      ].filter(Boolean).join(' ').replace(/\s+/g, ' ').trim().toLowerCase();
      return /stop (?:generating|streaming|response)|停止(?:生成|產生|回答|回覆)/.test(label);
    });
  }

  function messageObservation(messageAdapter) {
    if (messageAdapter && typeof messageAdapter.lastAssistantObservation === 'function') {
      return messageAdapter.lastAssistantObservation();
    }
    return {
      key: '',
      fingerprint: '',
      pending: !!(messageAdapter && typeof messageAdapter.lastAssistantPending === 'function' &&
        messageAdapter.lastAssistantPending()),
      completionVisible: false
    };
  }

  function readState(policy, messageAdapter, document, composer, visible) {
    const observation = messageObservation(messageAdapter);
    const active = officialActive(document, composer, visible);
    return policy
      ? policy.observe(observation, active)
      : { active: active || observation.pending, assistantKey: observation.key };
  }

  function begin(policy, messageAdapter, options) {
    return policy ? policy.begin(messageObservation(messageAdapter), options) : null;
  }

  return Object.freeze({ create, officialActive, messageObservation, readState, begin });
});

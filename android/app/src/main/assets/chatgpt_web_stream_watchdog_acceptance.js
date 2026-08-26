(function (root, factory) {
  'use strict';

  const api = factory();
  if (typeof module !== 'undefined' && module.exports) module.exports = api;
  if (root) root.__elonChatGptStreamWatchdogAcceptance = Object.freeze(api);
})(typeof window !== 'undefined' ? window : null, function () {
  'use strict';

  function create(options) {
    const probeModule = options.probeModule;
    const streamingPolicy = options.streamingPolicy;
    const onResult = options.onResult;
    const scheduleSnapshot = options.scheduleSnapshot;
    const probe = probeModule && probeModule.create({
      now: options.now,
      scheduleTimer: options.scheduleTimer,
      cancelTimer: options.cancelTimer,
      onResult
    });

    function reject(requestId, detail, report) {
      if (probe) probe.dispose();
      if (streamingPolicy && typeof streamingPolicy.reset === 'function') streamingPolicy.reset();
      if (report === true && typeof onResult === 'function') onResult(requestId, false, detail);
      return { accepted: false, detail };
    }

    function run(requestId, streaming) {
      if (!probe || !streamingPolicy || typeof streamingPolicy.scheduleNext !== 'function') {
        return reject(requestId, 'private_stream_watchdog_probe_unavailable');
      }
      const armed = probe.arm(requestId, streaming);
      if (!armed || armed.accepted !== true) return armed;
      if (probe.observePrivateUpdate('streaming') !== false) {
        return reject(requestId, 'private_stream_watchdog_probe_unavailable');
      }
      const scheduled = streamingPolicy.scheduleNext(
        true,
        (schedule) => {
          if (!probe.watchdogFired(schedule)) {
            reject(requestId, 'private_stream_watchdog_schedule_rejected', true);
          }
          if (typeof scheduleSnapshot === 'function') scheduleSnapshot();
        },
        { privateStreamObserved: true }
      );
      if (!scheduled || scheduled.mode !== 'private_stream_watchdog') {
        return reject(requestId, 'private_stream_watchdog_schedule_unavailable');
      }
      return { accepted: true, detail: '' };
    }

    function dispose() {
      if (probe) probe.dispose();
    }

    return Object.freeze({ run, dispose });
  }

  return Object.freeze({ create });
});

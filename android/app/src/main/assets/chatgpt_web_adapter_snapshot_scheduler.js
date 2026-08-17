(function (root) {
  'use strict';

  function create(options) {
    const scheduleTimer = options.scheduleTimer;
    const cancelTimer = options.cancelTimer;
    const snapshot = options.snapshot;
    const quietDelayMs = Number(options.quietDelayMs) || 120;
    const maxDelayMs = Number(options.maxDelayMs) || 1000;
    const activeQuietDelayMs = Number(options.activeQuietDelayMs) || quietDelayMs;
    const activeMaxDelayMs = Number(options.activeMaxDelayMs) || maxDelayMs;
    let quietTimer = 0;
    let maxTimer = 0;
    let maxTimerDelayMs = 0;
    let disposed = false;

    function cancelPending() {
      if (quietTimer) cancelTimer(quietTimer);
      if (maxTimer) cancelTimer(maxTimer);
      quietTimer = 0;
      maxTimer = 0;
      maxTimerDelayMs = 0;
    }

    function emit() {
      if (disposed) return;
      cancelPending();
      snapshot();
    }

    function schedule(active) {
      if (disposed) return;
      const nextQuietDelayMs = active === true ? activeQuietDelayMs : quietDelayMs;
      const nextMaxDelayMs = active === true ? activeMaxDelayMs : maxDelayMs;
      if (quietTimer) cancelTimer(quietTimer);
      quietTimer = scheduleTimer(nextQuietDelayMs, emit);
      if (!maxTimer || nextMaxDelayMs < maxTimerDelayMs) {
        if (maxTimer) cancelTimer(maxTimer);
        maxTimer = scheduleTimer(nextMaxDelayMs, emit);
        maxTimerDelayMs = nextMaxDelayMs;
      }
    }

    function dispose() {
      disposed = true;
      cancelPending();
    }

    return Object.freeze({ schedule, dispose });
  }

  root.__elonChatGptSnapshotScheduler = Object.freeze({ create });
})(window);

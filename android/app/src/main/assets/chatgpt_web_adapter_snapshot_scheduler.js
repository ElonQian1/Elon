(function (root) {
  'use strict';

  function create(options) {
    const scheduleTimer = options.scheduleTimer;
    const cancelTimer = options.cancelTimer;
    const snapshot = options.snapshot;
    const quietDelayMs = Number(options.quietDelayMs) || 120;
    const maxDelayMs = Number(options.maxDelayMs) || 1000;
    let quietTimer = 0;
    let maxTimer = 0;
    let disposed = false;

    function cancelPending() {
      if (quietTimer) cancelTimer(quietTimer);
      if (maxTimer) cancelTimer(maxTimer);
      quietTimer = 0;
      maxTimer = 0;
    }

    function emit() {
      if (disposed) return;
      cancelPending();
      snapshot();
    }

    function schedule() {
      if (disposed) return;
      if (quietTimer) cancelTimer(quietTimer);
      quietTimer = scheduleTimer(quietDelayMs, emit);
      if (!maxTimer) maxTimer = scheduleTimer(maxDelayMs, emit);
    }

    function dispose() {
      disposed = true;
      cancelPending();
    }

    return Object.freeze({ schedule, dispose });
  }

  root.__elonChatGptSnapshotScheduler = Object.freeze({ create });
})(window);

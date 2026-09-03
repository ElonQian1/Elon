(function (root, factory) {
  'use strict';

  const api = factory(root);
  if (typeof module === 'object' && module.exports) {
    module.exports = Object.freeze(Object.assign({ createForRoot: factory }, api));
  }
  if (root && root.location && root.location.origin === 'https://chatgpt.com') {
    const current = root.__elonChatGptDictationActions;
    if (!current || Number(current.version) < Number(api.version)) {
      root.__elonChatGptDictationActions = Object.freeze(api);
    }
  }
})(typeof window === 'object' ? window : globalThis, function (root) {
  'use strict';

  const VERSION = 1;

  function unavailableCapture() {
    return Object.freeze({
      arm() {},
      finish() {},
      active: () => false,
      pending: () => false,
      waitForActive: () => Promise.resolve(false),
      waitForInactive: () => Promise.resolve(false)
    });
  }

  function create(dependencies) {
    const deps = dependencies || {};
    const runtime = root && root.__elonChatGptDictationRuntime;
    const capture = runtime && typeof runtime.createCaptureTracker === 'function'
      ? runtime.createCaptureTracker()
      : unavailableCapture();

    function sessionButton(kind) {
      return typeof deps.findSessionButton === 'function'
        ? deps.findSessionButton(kind)
        : null;
    }

    function start(composer, emitEvent, result) {
      const button = typeof deps.findStartButton === 'function'
        ? deps.findStartButton(composer)
        : null;
      if (!button) return result('start_dictation', false, '官网当前没有听写入口。');
      capture.arm();
      const research = root && root.__elonChatGptRealtimeVoiceResearch;
      if (research && typeof research.activate === 'function') {
        try { research.activate(); } catch (_) {}
      }
      const touched = typeof deps.emitStartTouch === 'function' &&
        deps.emitStartTouch('start_dictation', button, emitEvent);
      if (!touched) {
        const layout = root && root.__elonChatGptLayout;
        if (!layout || typeof layout.requestSemanticTouch !== 'function' ||
            !layout.requestSemanticTouch('dictation', 'start_dictation', emitEvent, 'composer')) {
          capture.finish();
          return result('start_dictation', false, '官网听写入口当前不可见。');
        }
      }
      const wait = runtime && typeof runtime.waitUntil === 'function'
        ? runtime.waitUntil(() => {
          if (capture.active()) return true;
          return !!sessionButton('cancel') && !!sessionButton('submit');
        }, 8000)
        : capture.waitForActive(8000);
      return Promise.resolve(wait).then((confirmed) => {
        if (!confirmed) capture.finish();
        result(
          'start_dictation',
          confirmed === true,
          confirmed === true ? 'capture_started' : 'dictation_start_unconfirmed'
        );
      });
    }

    function finish(kind, emitEvent, result) {
      const action = kind === 'cancel' ? 'cancel_dictation' : 'submit_dictation';
      const button = sessionButton(kind);
      if (!button) return result(action, false, '官网当前没有进行中的听写。');
      if (typeof deps.emitSessionTouch !== 'function' ||
          !deps.emitSessionTouch(action, button, emitEvent)) {
        return result(action, false, '官网听写操作当前不可见。');
      }
      const wait = runtime && typeof runtime.waitUntil === 'function'
        ? runtime.waitUntil(() => {
          const controlsGone = !sessionButton('cancel') && !sessionButton('submit');
          return controlsGone && !capture.active() && !capture.pending();
        }, 10000)
        : capture.waitForInactive(10000);
      return Promise.resolve(wait).then((confirmed) => {
        if (confirmed) capture.finish();
        result(
          action,
          confirmed === true,
          confirmed === true ? 'capture_finished' : 'dictation_finish_unconfirmed'
        );
      });
    }

    return Object.freeze({
      captureActive: capture.active,
      capturePending: capture.pending,
      start,
      cancel: (emitEvent, result) => finish('cancel', emitEvent, result),
      submit: (emitEvent, result) => finish('submit', emitEvent, result)
    });
  }

  return Object.freeze({ version: VERSION, create });
});

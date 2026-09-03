(function (root, factory) {
  'use strict';

  const api = Object.freeze({ version: 1, create: factory });
  if (typeof module === 'object' && module.exports) module.exports = api;
  if (root) root.__elonChatGptPrivateDictationOrchestrator = api;
})(typeof window === 'object' ? window : globalThis, function (options) {
  'use strict';

  const config = options || {};
  const transport = config.transport;
  const findComposer = config.findComposer;
  const composerValue = config.composerValue;
  const setComposerValue = config.setComposerValue;
  const comparableText = config.comparableText;
  const scheduleSnapshot = config.scheduleSnapshot;

  function supported() {
    return !!(transport && typeof transport.ready === 'function' &&
      typeof transport.start === 'function' && typeof transport.submit === 'function' &&
      typeof transport.cancel === 'function');
  }

  function appendTranscript(baseValue, transcriptValue) {
    const base = String(baseValue || '');
    const transcript = String(transcriptValue || '').trim();
    if (!base) return transcript;
    if (!transcript || /\s$/.test(base)) return base + transcript;
    const needsSpace = /[A-Za-z0-9]$/.test(base) && /^[A-Za-z0-9]/.test(transcript);
    return base + (needsSpace ? ' ' : '') + transcript;
  }

  function detail(outcome) {
    return String(outcome && outcome.code || 'private_dictation_failed').slice(0, 120);
  }

  function start(nativeDraft, expectedOfficialDraft, respond) {
    if (!supported() || !transport.ready()) {
      return respond('private_start_dictation', false, 'before_capture:unavailable');
    }
    const composer = findComposer();
    if (!composer) return respond('private_start_dictation', false, 'before_capture:composer_missing');
    const current = composerValue(composer);
    if (comparableText(current) !== comparableText(expectedOfficialDraft)) {
      return respond('private_start_dictation', false, 'before_capture:draft_changed');
    }
    if (comparableText(current) !== comparableText(nativeDraft) &&
        !setComposerValue(composer, nativeDraft)) {
      return respond('private_start_dictation', false, 'before_capture:draft_rejected');
    }
    return Promise.resolve(transport.start()).then((outcome) => {
      respond('private_start_dictation', outcome && outcome.ok === true, detail(outcome));
      if (typeof scheduleSnapshot === 'function') scheduleSnapshot(true);
    }, () => respond('private_start_dictation', false, 'before_capture:exception'));
  }

  function submit(respond) {
    if (!supported()) return respond('private_submit_dictation', false, 'capture:unavailable');
    return Promise.resolve(transport.submit()).then((outcome) => {
      if (!outcome || outcome.ok !== true || typeof outcome.transcript !== 'string') {
        respond('private_submit_dictation', false, detail(outcome));
        return;
      }
      const composer = findComposer();
      if (!composer) {
        respond('private_submit_dictation', false, 'capture:composer_missing');
        return;
      }
      const nextDraft = appendTranscript(composerValue(composer), outcome.transcript);
      if (!setComposerValue(composer, nextDraft)) {
        respond('private_submit_dictation', false, 'capture:draft_rejected');
        return;
      }
      respond(
        'private_submit_dictation',
        true,
        'transcript_ready:' + Math.max(0, Number(outcome.transcriptLength) || outcome.transcript.length)
      );
      if (typeof scheduleSnapshot === 'function') scheduleSnapshot(true);
    }, () => respond('private_submit_dictation', false, 'capture:exception'));
  }

  function cancel(respond) {
    if (!supported()) return respond('private_cancel_dictation', false, 'capture:unavailable');
    return Promise.resolve(transport.cancel()).then((outcome) => {
      respond('private_cancel_dictation', outcome && outcome.ok === true, detail(outcome));
      if (typeof scheduleSnapshot === 'function') scheduleSnapshot(true);
    }, () => respond('private_cancel_dictation', false, 'capture:exception'));
  }

  return Object.freeze({ start, submit, cancel, appendTranscript });
});

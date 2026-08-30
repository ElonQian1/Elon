(function () {
  'use strict';

  const existing = window.__elonChatGptTextTransactionOrchestrator;
  if (existing && Number(existing.version) >= 1) return;

  const SEND_BUTTON_POLL_MS = 60;
  const SEND_BUTTON_SETTLE_MS = 180;
  const SEND_BUTTON_TIMEOUT_MS = 4000;
  const SEND_ACCEPT_TIMEOUT_MS = 3000;

  function create(options) {
    if (!options || typeof options.findComposer !== 'function' ||
        typeof options.composerValue !== 'function' ||
        typeof options.setComposerValue !== 'function' ||
        typeof options.comparableText !== 'function' ||
        typeof options.scheduleSnapshot !== 'function') return null;
    const privateTextTransactionRelay = window.__elonChatGptPrivateTextTransactionRelay;
    const privateStreamTransport = window.__elonChatGptPrivateStreamTransport;
    const privateSendObserver = window.__elonChatGptPrivateSendObserver;

    function safeCode(value, fallback) {
      const code = String(value || fallback).replace(/[^a-z_]/g, '').slice(0, 32);
      return code || fallback;
    }

    function findSendButton(composer) {
      const scope = composer && composer.closest('form');
      const candidates = [
        scope && scope.querySelector('[data-testid="send-button"]'),
        scope && scope.querySelector('button[aria-label*="send" i]'),
        scope && scope.querySelector('button[type="submit"]'),
        options.findButton('send-button', ['send', '发送'])
      ];
      return candidates.find((button) =>
        options.isVisible(button) && !button.disabled &&
        button.getAttribute('aria-disabled') !== 'true'
      ) || null;
    }

    function waitForStableSendButton(composer, expectedValue, onReady, onTimeout) {
      const started = Date.now();
      let readySince = 0;
      let readyButton = null;
      function poll() {
        const button = findSendButton(composer);
        const draftMatches = options.comparableText(options.composerValue(composer)) ===
          options.comparableText(expectedValue);
        if (button && draftMatches) {
          if (button !== readyButton) {
            readyButton = button;
            readySince = Date.now();
          }
          if (Date.now() - readySince >= SEND_BUTTON_SETTLE_MS) return onReady(button);
        } else {
          readyButton = null;
          readySince = 0;
        }
        if (Date.now() - started >= SEND_BUTTON_TIMEOUT_MS) return onTimeout();
        window.setTimeout(poll, SEND_BUTTON_POLL_MS);
      }
      poll();
    }

    function waitForSendAccepted(composer, expectedValue, sendMarker, onAccepted, onTimeout) {
      const started = Date.now();
      function poll() {
        if (privateSendObserver && typeof privateSendObserver.dispatchedAfter === 'function' &&
            privateSendObserver.dispatchedAfter(sendMarker)) {
          return onAccepted('official_request_dispatched');
        }
        const currentValue = options.comparableText(options.composerValue(composer));
        if (!currentValue || currentValue !== options.comparableText(expectedValue) ||
            options.readStreamingState().active) {
          return onAccepted('official_page_accepted');
        }
        if (Date.now() - started >= SEND_ACCEPT_TIMEOUT_MS) return onTimeout();
        window.setTimeout(poll, SEND_BUTTON_POLL_MS);
      }
      poll();
    }

    function tryPrivateSend(composer, value, expectedDraft, assistantBeforeSend, respond) {
      if (!privateTextTransactionRelay ||
          typeof privateTextTransactionRelay.dispatch !== 'function') {
        return { handled: false, code: 'relay_unavailable' };
      }
      if (!privateStreamTransport ||
          typeof privateStreamTransport.preparePrivateSend !== 'function') {
        return { handled: false, code: 'stream_unavailable' };
      }
      const prompt = options.comparableText(value);
      const draft = options.comparableText(expectedDraft);
      if (draft && draft !== prompt) return { handled: false, code: 'draft_mismatch' };
      if (composer && options.comparableText(options.composerValue(composer)) !== draft) {
        return { handled: false, code: 'draft_changed' };
      }
      if (draft && !composer) return { handled: false, code: 'draft_unavailable' };
      let transaction;
      try {
        transaction = privateTextTransactionRelay.dispatch({
          prompt: value,
          requestId: respond.requestId || ''
        });
      } catch (_) {
        return { handled: false, code: 'relay_exception' };
      }
      if (!transaction || transaction.dispatched !== true) {
        return { handled: false, code: safeCode(transaction && transaction.code, 'not_ready') };
      }
      if (draft && (!composer || options.comparableText(options.composerValue(composer)) !== draft ||
          !options.setComposerValue(composer, ''))) {
        respond('send_prompt', false, 'private_text_v1:unknown:draft_handoff');
        options.scheduleSnapshot(true);
        return { handled: true, code: '' };
      }
      if (!transaction.completion ||
          !privateStreamTransport.preparePrivateSend(value, transaction.userMessageId)) {
        respond('send_prompt', false, 'private_text_v1:unknown:state_handoff');
        return { handled: true, code: '' };
      }
      if (options.streamingPolicy) options.streamingPolicy.begin(assistantBeforeSend);
      options.scheduleSnapshot(true);
      Promise.resolve(transaction.completion).then((receipt) => {
        if (receipt && receipt.status === 'accepted') {
          respond('send_prompt', true, 'private_text_v1:accepted');
        } else {
          respond('send_prompt', false,
            'private_text_v1:unknown:' + safeCode(receipt && receipt.code, 'unknown'));
        }
        options.scheduleSnapshot(true);
      }).catch(() => {
        respond('send_prompt', false, 'private_text_v1:unknown:network');
        options.scheduleSnapshot(true);
      });
      return { handled: true, code: '' };
    }

    function sendPrompt(value, expectedDraft, respond, allowPrivateTextTransaction) {
      const composer = options.findComposer();
      const assistantBeforeSend = options.streamingPolicyModule &&
        options.streamingPolicyModule.messageObservation(options.messageAdapter);
      let privateFallbackCode = '';
      if (allowPrivateTextTransaction === true) {
        const attempt = tryPrivateSend(
          composer, value, expectedDraft, assistantBeforeSend, respond
        );
        if (attempt.handled) return;
        privateFallbackCode = attempt.code;
      }
      if (!composer) return respond('send_prompt', false, '未找到输入框，请切换网页模式。');
      if (options.comparableText(options.composerValue(composer)) !==
          options.comparableText(expectedDraft)) {
        return respond('send_prompt', false, '网页草稿已变化，请返回官网确认后重试。');
      }
      if (!options.setComposerValue(composer, value)) {
        return respond('send_prompt', false, '官方输入框未接受文本，请返回官网重试。');
      }
      waitForStableSendButton(composer, value, (button) => {
        const sendMarker = privateSendObserver && typeof privateSendObserver.marker === 'function'
          ? privateSendObserver.marker()
          : null;
        if (privateStreamTransport && typeof privateStreamTransport.prepareSend === 'function') {
          privateStreamTransport.prepareSend();
        }
        button.click();
        options.scheduleSnapshot(true);
        waitForSendAccepted(composer, value, sendMarker, (acceptance) => {
          if (options.streamingPolicy) options.streamingPolicy.begin(assistantBeforeSend);
          respond(
            'send_prompt',
            true,
            (acceptance === 'official_request_dispatched'
              ? '官网发送请求已提交。'
              : '官方网页已确认发送。') +
              (privateFallbackCode ? ' [private_fallback:' + privateFallbackCode + ']' : '')
          );
          options.scheduleSnapshot();
        }, () => respond('send_prompt', false, '官方网页未确认发送，请重试。'));
      }, () => respond('send_prompt', false, '发送按钮尚未就绪，请返回官网重试。'));
    }

    function tryPrivateRegeneration(respond) {
      if (!privateTextTransactionRelay ||
          typeof privateTextTransactionRelay.dispatchRegenerate !== 'function' ||
          !privateStreamTransport ||
          typeof privateStreamTransport.preparePrivateRegeneration !== 'function') return false;
      let transaction;
      try {
        transaction = privateTextTransactionRelay.dispatchRegenerate({
          requestId: respond.requestId || ''
        });
      } catch (_) {
        return false;
      }
      if (!transaction || transaction.dispatched !== true) return false;
      if (!privateStreamTransport.preparePrivateRegeneration()) {
        respond('regenerate_response', false, 'private_text_v1:unknown:state_handoff');
        return true;
      }
      if (options.streamingPolicy) options.streamingPolicy.begin(
        options.streamingPolicyModule &&
          options.streamingPolicyModule.messageObservation(options.messageAdapter)
      );
      options.scheduleSnapshot(true);
      Promise.resolve(transaction.completion).then((receipt) => {
        if (receipt && receipt.status === 'accepted') {
          respond('regenerate_response', true, 'private_text_v1:regenerate_accepted');
        } else {
          respond('regenerate_response', false,
            'private_text_v1:regenerate_unknown:' + safeCode(receipt && receipt.code, 'unknown'));
        }
        options.scheduleSnapshot(true);
      }).catch(() => {
        respond('regenerate_response', false, 'private_text_v1:regenerate_unknown:network');
        options.scheduleSnapshot(true);
      });
      return true;
    }

    function stopPrivate(respond) {
      if (!privateTextTransactionRelay ||
          typeof privateTextTransactionRelay.stop !== 'function' ||
          !privateTextTransactionRelay.stop()) return false;
      if (privateStreamTransport &&
          typeof privateStreamTransport.finishPrivateSend === 'function') {
        privateStreamTransport.finishPrivateSend();
      }
      respond('stop_generation', true, 'private_text_v1:stopped');
      options.scheduleSnapshot(true);
      return true;
    }

    return Object.freeze({ sendPrompt, tryPrivateRegeneration, stopPrivate });
  }

  window.__elonChatGptTextTransactionOrchestrator = Object.freeze({ version: 1, create });
})();

(function (root, factory) {
  'use strict';
  const api = Object.freeze({ version: 5, create: factory });
  if (typeof module === 'object' && module.exports) module.exports = api;
  if (root?.location?.origin === 'https://chatgpt.com' && !root.__elonChatGptRspackSubmit) {
    root.__elonChatGptRspackSubmit = factory(root);
  }
})(typeof window === 'object' ? window : null, function (page, options) {
  'use strict';
  options = options || {};
  const runtime = page.__elonChatGptRspackRuntime;
  const context = (options.context || page.__elonChatGptRspackContext)?.create(page);
  let active = null, code = 'not_observed', requestGuard = null;
  function submit(command) {
    if (!runtime?.observed()) return { handled: false, code: 'not_observed' };
    if (page.__elonChatGptPrivateTextTransactionsEnabled !== true) return { handled: false, code: 'disabled' };
    if (active) return { handled: true, completion: Promise.resolve({ status: 'unknown', code: 'busy' }) };
    if (typeof command?.prompt !== 'string' || !command.prompt.trim() || command.prompt.length > 20000 ||
        typeof command.expectedDraft !== 'string' || !/^mcp_[a-z0-9]{1,32}$/.test(command.requestId || '')) {
      return { handled: false, code: 'invalid_command' };
    }
    const job = { invoked: false, uncertain: false, token: page.__elonChatGptDocumentToken };
    active = job;
    const completion = run(command, job).finally(() => { if (active === job && !job.uncertain) active = null; });
    return { handled: true, completion };
  }
  async function run(command, job) {
    const reject = reason => { code = reason; return { status: 'rejected', code: reason }; };
    let timer, controller;
    try {
      code = 'loading';
      if (!context || !await runtime.load()) return reject(runtime.state().code);
      if (job.token !== page.__elonChatGptDocumentToken) return reject('document_changed');
      const attachmentLease = page.__elonChatGptRspackAttachments?.prepare(command.composer);
      if (!attachmentLease && page.__elonChatGptRspackAttachments?.state().count > 0) {
        return reject(page.__elonChatGptRspackAttachments.state().code);
      }
      if (command.requireNativeAttachment && !attachmentLease) return reject('attachment_contract_pending');
      const binding = attachmentLease?.binding || context.capture(command.composer);
      if (!binding) return reject(context.state().code);
      // An empty contenteditable paragraph reports a layout newline via innerText.
      // Only normalize that empty state; never trim or overwrite a real draft.
      const draftMatches = () => {
        const value = command.readDraft();
        return value === command.expectedDraft || command.expectedDraft === '' &&
          typeof value === 'string' && value.trim() === '';
      };
      if (binding.draft !== command.expectedDraft || !draftMatches() ||
          command.expectedDraft && command.expectedDraft !== command.prompt) return reject('draft_mismatch');
      command.beforeSubmit?.();
      if (!context.current(binding) || attachmentLease?.current() === false ||
          !draftMatches()) return reject('context_changed');
      controller = new page.AbortController();
      const current = () => !controller.signal.aborted &&
        (job.serverId ? context.requestCurrent(binding, job.serverId) : context.owns(binding)) &&
        (job.accepted || !!job.serverId || attachmentLease?.current() !== false);
      requestGuard = page.__elonChatGptGroupRequestOwnershipEnabled === true
        ? () => !controller.signal.aborted && context.ownedRequestCurrent(binding, job.serverId) : current;
      code = 'dispatching';
      job.invoked = true;
      // CUv.a is the reviewed official composer transaction. It prepares integrity,
      // updates AppScope, and starts the official stream without clicking a DOM button.
      const request = binding.runtime.submit.a(binding.scope, {
        conversationId: binding.id, prompt: command.prompt, selectedModel: binding.model,
        isTemporaryChat: binding.temporary, preserveDraft: true, requireDispatchAcceptance: true,
        ...(attachmentLease ? { additionalAttachments: attachmentLease.attachments } : {}),
        isSubmissionCurrent: current, isRequestCurrent: requestGuard, signal: controller.signal,
        onServerThreadIdChange: id => { job.serverId = id; },
      });
      const accepted = await Promise.race([Promise.resolve(request), new Promise((_, reject) => {
        timer = page.setTimeout(() => { controller.abort(); reject(Error('timeout')); }, options.timeoutMs || 20000);
      })]);
      if (accepted !== true) {
        // The official function can return false after a network error: no replay.
        job.uncertain = true; code = 'dispatch_unconfirmed';
        return { status: 'unknown', code };
      }
      job.accepted = true;
      const sameOwner = context.owns(binding);
      try {
        attachmentLease?.consumeAccepted();
        if (sameOwner && command.expectedDraft && command.readDraft() === command.expectedDraft) command.clearDraft?.();
      } catch (_) { /* Local cleanup cannot revoke a confirmed dispatch. */ }
      code = 'accepted';
      return { status: 'accepted', code, current: sameOwner };
    } catch (_) {
      code = job.invoked ? 'dispatch_unconfirmed' : 'context_unavailable';
      job.uncertain = job.invoked;
      return { status: job.invoked ? 'unknown' : 'rejected', code };
    } finally { page.clearTimeout(timer); }
  }
  async function inspect(node) {
    try {
      const loaded = await runtime.load();
      const lease = loaded && page.__elonChatGptRspackAttachments?.prepare(node);
      const binding = loaded && (lease?.binding || context?.capture(node));
      const attachmentState = page.__elonChatGptRspackAttachments?.state();
      const reason = binding ? 'ready' : loaded ? attachmentState?.count > 0 && !lease ? attachmentState.code
        : context?.state().code || 'context_unavailable' : runtime.state().code;
      return { profile: runtime.profile, stage: binding ? 'ready' : 'runtime', code: reason };
    } catch (_) { return { profile: runtime.profile, stage: 'runtime', code: 'context_unavailable' }; }
  }
  return Object.freeze({ version: 5, submit, inspect, state: () => ({ pending: !!active, code,
    requestCurrent: requestGuard ? requestGuard() : null }) });
});

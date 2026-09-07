(function (root, factory) {
  'use strict';
  const api = Object.freeze({ version: 1, create: factory });
  if (typeof module === 'object' && module.exports) module.exports = api;
  if (root?.location?.origin === 'https://chatgpt.com') root.__elonChatGptNewChatConfirmation = factory(root);
})(typeof window === 'object' ? window : null, function (page) {
  'use strict';
  const paths = page.__elonChatGptCommittedOwnerPath ||
    (typeof module === 'object' && module.exports ? require('./chatgpt_web_committed_owner_path') : null);
  // Exact registered modal callbacks in the two inspected public website builds.
  // They preserve the official router and close lifecycle; no DOM click or reset POST.
  const specs = {
    web_20260906: { owner: 'xca', close: '()=>{Sm.closeModal(ym.NoAuthNewChat)}',
      clear: '()=>{let n=`chatgpt_new_chat_modal_new_chat_button_clicked`,r=q.logStructuredEvent(`ChatgptNewChatModalClearConfirmed`,{eventName:n});q.logValueEventWithStatsig({segmentEventName:n,statsigEventName:n,eventId:r,statsigMetadata:{}}),Cjt({location:`New Chat Modal New Chat Button`}),UT(t),e()}' },
    web_20260907: { owner: 'Mla', close: '()=>{sm.closeModal(im.NoAuthNewChat)}',
      clear: '()=>{let n=`chatgpt_new_chat_modal_new_chat_button_clicked`,r=H.logStructuredEvent(`ChatgptNewChatModalClearConfirmed`,{eventName:n});H.logValueEventWithStatsig({segmentEventName:n,statsigEventName:n,eventId:r,statsigMetadata:{}}),Zjt({location:`New Chat Modal New Chat Button`}),lT(t),e()}' }
  };
  const source = fn => typeof fn === 'function' ? Function.prototype.toString.call(fn) : '';

  function capture() {
    try {
      const bindings = page.__elonChatGptPrivateRuntimeBindings;
      const profile = bindings?.state?.().profile_id, spec = specs[profile], shared = bindings?.peek('shared');
      if (!spec || !paths || typeof shared?.R5 !== 'function' || typeof shared.F5 !== 'function' ||
          shared.R5()?.authStatus !== 'logged_out' || shared.F5() !== null) return null;
      const modal = page.document.querySelector('[data-testid="modal-no-auth-new-chat"]');
      if (!modal?.isConnected) return null;
      const buttons = modal.querySelectorAll('button');
      if (!buttons.length || buttons.length > 16) return null;
      const matches = [];
      for (const button of buttons) {
        if (!button.isConnected || button.disabled || button.getAttribute('aria-disabled') === 'true') continue;
        const keys = Object.keys(button).filter(key => key.startsWith('__reactFiber$'));
        if (keys.length !== 1) continue;
        const chain = paths.resolve(button[keys[0]]).ancestors;
        const owners = chain.filter(fiber => fiber.type?.name === spec.owner);
        if (owners.length !== 1 || source(owners[0].memoizedProps?.onClose) !== spec.close) continue;
        const index = chain.indexOf(owners[0]);
        const actions = new Set(chain.slice(0, index).map(fiber => fiber.memoizedProps)
          .filter(props => props?.color === 'primary' && props.size === 'large' &&
            props.children?.props?.id === 'cBGbrD' && source(props.onClick) === spec.clear)
          .map(props => props.onClick));
        if (actions.size === 1) matches.push({ modal, profile, clear: [...actions][0], close: owners[0].memoizedProps.onClose });
      }
      return matches.length === 1 ? matches[0] : null;
    } catch (_) { return null; }
  }

  function run(before, decision) {
    const current = capture();
    if (!current || current.modal !== before?.modal || current.profile !== before.profile ||
        !['confirm', 'cancel'].includes(decision)) return false;
    // The caller consumes its one-use user decision before invoking this callback.
    if (decision === 'confirm') current.clear();
    else current.close();
    return true;
  }

  return Object.freeze({ version: 1, capture, run });
});

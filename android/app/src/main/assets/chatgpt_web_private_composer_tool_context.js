(function (root, capture) {
  'use strict';
  const api = Object.freeze({ version: 1, capture });
  if (typeof module === 'object' && module.exports) module.exports = api;
  if (root?.location?.origin === 'https://chatgpt.com') root.__elonChatGptPrivateComposerToolContext = api;
})(typeof window === 'object' ? window : null, function (page, tools) {
  'use strict';
  const spec = page.__elonChatGptPrivateRuntimeBindings?.tools?.();
  const ownerPath = page.__elonChatGptCommittedOwnerPath ||
    (typeof module === 'object' && module.exports ? require('./chatgpt_web_committed_owner_path') : null);
  const input = page.document.querySelector('#prompt-textarea');
  const context = page.__elonChatGptPrivateTextRuntimeSubmit?.captureConversation?.(input, true);
  const node = page.document.querySelector('#composer-plus-btn');
  if (!spec || !context || !node?.isConnected) return null;
  const key = Object.keys(node).find(name => name.startsWith('__reactFiber$'));
  const owners = ownerPath?.resolve(node[key])?.ancestors?.filter(fiber => fiber.type?.name === spec.owner) || [];
  if (owners.length !== 1) return null;
  const owner = owners[0], props = owner.memoizedProps;
  if (props?.conversation !== context.conversation || props.composerController !== context.controller ||
      props.composerDisabled !== false || props.isTemporaryChat !== context.temporary ||
      props.composerToolAvailability && props.composerToolAvailability !== 'default' ||
      props.loginModalGate?.shouldGateToLoginModal ||
      !Array.isArray(props.availableSystemHints) || typeof props.selectModelId !== 'function' ||
      typeof props.clearModelSelection !== 'function') return null;
  const model = props.currentModelId ?? props.currentModelConfig?.id;
  if (typeof model !== 'string' || !/^[a-z0-9][a-z0-9._-]{0,127}$/i.test(model)) return null;
  const data = owner.updateQueue?.memoCache?.data;
  const memo = Array.isArray(data) && data.length === 1 ? data[0] : null;
  // The inspected compiler output contains the already-filtered menu even when
  // its portal is closed. Do not rebuild account/model/voice eligibility rules.
  if (!Array.isArray(memo) || memo.length !== 265 || memo[56] !== props.availableSystemHints ||
      memo[63] !== false || memo[64] !== false || memo[65] !== false) return null;
  const menus = [];
  const seen = new Set();
  function visit(value, depth = 0) {
    if (!value || typeof value !== 'object' || seen.has(value) || depth > 6 || seen.size >= 48) return;
    seen.add(value);
    if (Array.isArray(value)) { value.forEach(item => visit(item, depth + 1)); return; }
    const p = value.props;
    if (!p) return;
    if (p.conversation === context.conversation && p.clearModelSelection === props.clearModelSelection &&
        Array.isArray(p.availableSystemHints) && typeof p.resolveSystemHintBehavior === 'function') {
      menus.push(p);
    }
    visit(p.children, depth + 1);
  }
  for (const slot of [90, 199]) visit(memo[slot]);
  if (!menus.length || menus.some(p => p.isLoading === true || p.isConsumerLockdownModeEnabled !== false)) return null;
  const available = [];
  for (const tool of tools) {
    const raw = props.availableSystemHints.filter(h => h?.systemHint === tool.hint);
    if (raw.length > 1) return null;
    const matches = menus.flatMap(p => p.availableSystemHints.filter(h => h?.systemHint === tool.hint)
      .map(hint => ({ hint, menu: p })));
    if (matches.length > 1) return null;
    if (!raw.length || !matches.length) continue;
    const { hint, menu } = matches[0];
    if ([raw[0], hint].some(h => h.isLoggedOutUpsell || h.isConnector || h.isDangerous ||
        h.hideFromInitialSelection || h.disabled || h.isDisabled)) continue;
    // A local action has a different transaction; never turn it into a hint write.
    if (menu.resolveSystemHintBehavior(hint) != null) continue;
    available.push(tool.hint);
  }
  if (!available.length) return null;
  return { controller: context.controller, conversation: context.conversation, model,
    href: context.href, token: context.token, account: context.account, guestProof: context.guestProof,
    document: page.document, node, shared: context.shared, serverId: context.serverId,
    allowed: available.join(',') };
});

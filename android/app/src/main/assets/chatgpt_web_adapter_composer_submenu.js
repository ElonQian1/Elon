(function (root, factory) {
  'use strict';

  const adapter = factory();
  if (typeof module !== 'undefined' && module.exports) module.exports = adapter;
  if (root) root.__elonChatGptComposerSubmenu = Object.freeze(adapter);
})(typeof window !== 'undefined' ? window : null, function () {
  'use strict';

  function containsNestedInteractiveControl(node) {
    return !!(node && node.querySelector && node.querySelector(
      '[role="menuitemradio"], [role="menuitemcheckbox"], [role="menuitem"], [role="option"]'
    ));
  }

  function withoutKnownOptionIds(options, knownIds) {
    if (!knownIds || knownIds.size === 0) return options;
    return options.filter((option) => !knownIds.has(option.id));
  }

  function matchingOption(options, identity) {
    return options.find((option) => option.id === identity.id) ||
      options.find((option) => option.label === identity.label) || null;
  }

  function createRecovery(dependencies) {
    const deps = dependencies || {};

    function recover(section, target, composer, emitEvent, result, scheduleSnapshot) {
      const action = section === 'model' ? 'select_model_option' : 'select_composer_tool';
      const parentIdentity = target.parentOption;
      const trigger = deps.triggerFor(section, composer);
      if (!parentIdentity || !trigger || !deps.isVisible(trigger)) {
        return result(action, false, '官网菜单已经关闭，请重新选择。');
      }
      const rootBaseline = deps.captureOptionBaseline();
      if (!deps.emitTriggerTouch(section, action, trigger, emitEvent)) {
        return result(action, false, '官网入口当前不可见。');
      }
      deps.waitForOptionsMatching(
        section,
        rootBaseline,
        (options) => !!matchingOption(options, parentIdentity),
        (rootOptions) => {
          const parent = matchingOption(rootOptions, parentIdentity);
          if (!parent || !deps.isOptionVisible(parent.node)) {
            return result(action, false, '官网模型分组已经变化，请重新选择。');
          }
          const childBaseline = deps.captureOptionBaseline();
          const rootOptionIds = new Set(rootOptions.map((option) => option.id));
          const submenuPurpose = section === 'model'
            ? 'open_model_submenu'
            : 'open_composer_tools_submenu';
          if (!deps.emitVisibleNodeTouch(submenuPurpose, parent.node, emitEvent)) {
            return result(action, false, '官网模型分组当前不可见。');
          }
          deps.waitForOptionsMatching(
            section,
            childBaseline,
            (options) => {
              const option = matchingOption(withoutKnownOptionIds(options, rootOptionIds), target);
              return !!option && !option.opensSubmenu;
            },
            (children) => {
              const nestedChildren = withoutKnownOptionIds(children, rootOptionIds);
              const recovered = matchingOption(nestedChildren, target);
              if (!recovered || recovered.opensSubmenu || !deps.isOptionVisible(recovered.node)) {
                return result(action, false, '官网目标选项已经变化，请重新选择。');
              }
              const contextualChildren = nestedChildren.map((option) => Object.assign(option, {
                parentOption: parentIdentity
              }));
              deps.emitOptions(section, contextualChildren, composer, emitEvent);
              if (!deps.emitVisibleNodeTouch(action, recovered.node, emitEvent)) {
                return result(action, false, '官网目标选项当前不可见。');
              }
              result(action, true, '');
              deps.schedule(scheduleSnapshot, 240);
            },
            () => result(action, false, '官网模型分组未返回目标选项。')
          );
        },
        () => result(action, false, '官网菜单未能重新打开。')
      );
    }

    return Object.freeze({ recover });
  }

  return Object.freeze({
    containsNestedInteractiveControl,
    createRecovery,
    withoutKnownOptionIds
  });
});

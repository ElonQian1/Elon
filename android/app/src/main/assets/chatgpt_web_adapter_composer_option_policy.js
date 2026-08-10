(function (root, factory) {
  'use strict';

  const policy = factory();
  if (typeof module === 'object' && module.exports) module.exports = policy;
  if (root) root.__elonChatGptComposerOptionPolicy = Object.freeze(policy);
})(typeof window === 'object' ? window : null, function () {
  'use strict';

  const FOREIGN_MENU_LABEL = /download\s+chatgpt|chatgpt\s+(desktop|mobile)|settings?|personalization|profile|log\s*out|sign\s*out|help|account|下载\s*chatgpt|桌面版|移动版|设置|个性化|个人资料|退出登录|帮助|账户|帐户|账号/iu;
  const MODEL_LABEL = /\b(gpt(?:-?[0-9a-z.]+)?|o[1-9](?:-[a-z0-9.]+)?|auto|instant|thinking|fast)\b|sol(?:轻度|重度)?|模型|自动|快速|思考|轻度|重度/iu;
  const OPTION_ROLES = new Set(['menuitem', 'menuitemcheckbox', 'menuitemradio', 'option']);

  function clean(value) {
    return String(value || '').replace(/\u00a0/g, ' ').replace(/\s+/g, ' ').trim();
  }

  function isForeignMenuLabel(label) {
    return FOREIGN_MENU_LABEL.test(clean(label));
  }

  function accepts(section, candidate) {
    const label = clean(candidate && candidate.label);
    const role = clean(candidate && candidate.role).toLowerCase();
    if (!label || isForeignMenuLabel(label)) return false;
    if (section === 'model') {
      return role === 'menuitemradio' || role === 'option' || MODEL_LABEL.test(label);
    }
    return section === 'tools' && (OPTION_ROLES.has(role) || !!(candidate && candidate.selectable));
  }

  function filter(section, candidates) {
    const values = Array.isArray(candidates) ? candidates : [];
    if (values.some((candidate) => isForeignMenuLabel(candidate && candidate.label))) return [];
    return values.filter((candidate) => accepts(section, candidate));
  }

  return Object.freeze({ accepts, filter, isForeignMenuLabel });
});

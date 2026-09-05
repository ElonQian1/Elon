(function (root, factory) {
  'use strict';

  const policy = factory(root && root.__elonChatGptModelLabelPolicy);
  if (typeof module === 'object' && module.exports) module.exports = policy;
  if (root) root.__elonChatGptComposerOptionPolicy = Object.freeze(policy);
})(typeof window === 'object' ? window : null, function (modelLabelPolicy) {
  'use strict';

  const FOREIGN_MENU_LABEL = /download\s+chatgpt|chatgpt\s+(desktop|mobile)|settings?|personalization|profile|log\s*out|sign\s*out|help|account|下载\s*chatgpt|桌面版|移动版|设置|个性化|个人资料|退出登录|帮助|账户|帐户|账号/iu;
  const MODEL_LABEL = /\b(gpt(?:-?[0-9a-z.]+)?|o[1-9](?:-[a-z0-9.]+)?|auto|instant|thinking|fast)\b|sol(?:轻度|重度)?|模型|能力|自动|快速|速度|思考|推理|轻度|标准|中度|重度|极高/iu;
  const EXPLICIT_TOOL_LABEL = /deep.?research|深度研究|create.?image|image.?generation|生成图片|创建图片|canvas|画布|study|学习|agent|代理模式|智能体|camera|take.?photo|相机|拍照|photos?|gallery|照片|相册|upload|files?|文件|上传|web.*search|search.*web|browse|搜索/iu;
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
      if (/^(?:能力|capabilit(?:y|ies))$/iu.test(label)) return false;
      const sharedModelMatch = modelLabelPolicy &&
        typeof modelLabelPolicy.isModelLabel === 'function' &&
        modelLabelPolicy.isModelLabel(label);
      return role === 'menuitemradio' || role === 'option' || sharedModelMatch || MODEL_LABEL.test(label);
    }
    if (section !== 'tools') return false;
    const sharedModelMatch = modelLabelPolicy &&
      typeof modelLabelPolicy.isModelLabel === 'function' &&
      modelLabelPolicy.isModelLabel(label);
    if (sharedModelMatch || MODEL_LABEL.test(label)) return false;
    return OPTION_ROLES.has(role) || !!(candidate && candidate.selectable);
  }

  function filter(section, candidates) {
    const values = Array.isArray(candidates) ? candidates : [];
    const accepted = values.filter((candidate) => accepts(section, candidate));
    if (section !== 'tools' || !values.some((candidate) => isForeignMenuLabel(candidate && candidate.label))) {
      return accepted;
    }
    return accepted.filter((candidate) =>
      (candidate && candidate.semantic && candidate.semantic !== 'tool') ||
      EXPLICIT_TOOL_LABEL.test(clean(candidate && candidate.label))
    );
  }

  return Object.freeze({ accepts, filter, isForeignMenuLabel });
});

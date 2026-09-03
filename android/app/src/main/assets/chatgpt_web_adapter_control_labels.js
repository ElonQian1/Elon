(function (root, factory) {
  'use strict';

  const api = factory();
  if (typeof module !== 'undefined' && module.exports) module.exports = api;
  if (root && !root.__elonChatGptControlLabels) root.__elonChatGptControlLabels = api;
})(typeof window !== 'undefined' ? window : globalThis, function () {
  'use strict';

  const LABELS = Object.freeze({
    navigation: '打开导航',
    title: '切换工作区',
    profile: '账户',
    new_conversation: '新建会话',
    temporary_chat: '临时聊天',
    attachment: '添加附件',
    image_generation: '创建图片',
    model: '选择模型',
    dictation: '开始听写',
    voice_mode: '启动语音功能',
    send: '发送',
    stop: '停止生成',
    suggestion: '使用建议',
    copy: '复制',
    regenerate: '重新生成',
    edit: '编辑',
    share: '分享',
    feedback: '反馈',
    read_aloud: '朗读',
    previous_response: '上一回复',
    next_response: '下一回复',
    branch: '创建分支',
    delete: '删除',
    close: '关闭',
    confirm: '确认',
    conversation: '打开会话',
    search: '搜索聊天',
    text_input: '输入内容',
    selection: '选择选项',
    toggle: '切换选项',
    slider: '调整数值',
    library: '文件库',
    apps: '应用',
    tasks: '任务',
    project: '项目',
    save_to_project: '保存到项目',
    gpts: 'GPT',
    settings: '设置',
    health: '健康',
    finances: '财务',
    work: '工作',
    create_asset: '创建文件或网站',
    sources: '文件和来源',
    conversation_files: '在聊天中查看文件',
    rename: '重命名会话',
    pin: '置顶聊天',
    archive: '归档',
    more: '更多操作',
    personalization: '个性化',
    help: '帮助',
    logout: '退出登录',
    plan: '套餐',
    open_media: '打开媒体',
    reasoning_details: '查看思考过程',
    timestamp: '消息时间'
  });

  function defaultLabel(semantic) {
    return LABELS[semantic] || '操作';
  }

  return Object.freeze({ defaultLabel });
});

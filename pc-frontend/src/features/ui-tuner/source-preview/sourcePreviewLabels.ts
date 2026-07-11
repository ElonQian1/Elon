import type { SourcePreviewNode } from './types'

interface SourcePreviewNodeLabels {
  primary: string
  type: string
  tooltip: string
}

const TYPE_LABELS: Record<string, string> = {
  view: '基础视图', viewgroup: '视图容器', linearlayout: '线性布局', framelayout: '层叠布局',
  relativelayout: '相对布局', constraintlayout: '约束布局', coordinatorlayout: '协调布局',
  scrollview: '滚动视图', horizontalscrollview: '横向滚动视图', nestedscrollview: '嵌套滚动视图',
  textview: '文本组件', edittext: '文本输入框', textinputedittext: '文本输入框', button: '按钮',
  materialbutton: '按钮', imagebutton: '图标按钮', imageview: '图片组件', recyclerview: '循环列表',
  listview: '列表', gridview: '网格列表', progressbar: '进度条', seekbar: '滑动条', switch: '开关',
  switchcompat: '开关', checkbox: '复选框', radiobutton: '单选按钮', radiogroup: '单选按钮组',
  spinner: '下拉选择框', space: '间隔占位', toolbar: '工具栏', cardview: '卡片',
  materialcardview: '卡片', webview: '网页视图', composeview: 'Compose 视图',
}

const TOKEN_LABELS: Record<string, string> = {
  accessibility: '无障碍', account: '账号', action: '操作', add: '添加', agent: '智能助手', ai: '人工智能',
  all: '全部', apk: '安装包', asr: '语音识别', attachment: '附件', attribution: '来源', avatar: '头像',
  back: '返回', bar: '栏', base: '基础', beam: '波束', bottom: '底部', btn: '按钮', bubble: '气泡',
  button: '按钮', cached: '缓存', cancel: '取消', chain: '链路', check: '检查', clear: '清除', cloud: '云端',
  condition: '条件', config: '配置', container: '容器', content: '内容', copy: '复制', custom: '自定义',
  day: '每日', default: '默认', details: '详情', edit: '编辑', engine: '引擎', engines: '引擎列表', error: '错误',
  evidence: '证据', fallback: '降级', feature: '功能', filter: '过滤', final: '最终', floating: '悬浮', friend: '好友',
  group: '分组', health: '健康状态', home: '首页', icon: '图标', id: '标识', indicator: '指示器', input: '输入',
  install: '安装', key: '密钥', label: '标签', language: '语言', last: '最后', layout: '布局', list: '列表',
  loading: '加载', login: '登录', manage: '管理', member: '成员', members: '成员', menu: '菜单', message: '消息',
  mode: '模式', model: '模型', msg: '消息', nickname: '昵称', open: '打开', output: '输出', overlay: '悬浮窗',
  password: '密码', pause: '暂停', period: '周期', plaza: '广场', plus: '加号', post: '动态', preset: '预设',
  probe: '检测', progress: '进度', project: '项目', radio: '单选', readiness: '就绪状态', register: '注册',
  reply: '回复', request: '申请', row: '行', save: '保存', scroll: '滚动区', search: '搜索', selection: '选择',
  server: '服务器', share: '分享', size: '大小', skip: '跳过', spinner: '下拉框', status: '状态', submit: '提交',
  summary: '摘要', switch: '开关', tab: '页签', text: '文本', timeline: '时间线', title: '标题', token: '令牌',
  tokens: '令牌', toggle: '切换', toolbar: '工具栏', top: '顶部', total: '总计', tts: '语音合成',
  update: '更新', usage: '用量', user: '用户', vad: '语音检测', voice: '语音', whisper: 'Whisper', work: '工作',
  wrap: '外层容器',
}

function simpleTag(tag: string) {
  const parts = tag.split('.')
  return parts[parts.length - 1] || tag
}

function componentType(node: SourcePreviewNode) {
  const rawName = node.resourceId ?? node.name
  if (/(button|btn|action)$/i.test(rawName)) return '按钮'
  if (/(input|edit)$/i.test(rawName)) return '输入框'
  if (/(tab|tabwrap)$/i.test(rawName)) return '页签'
  const fallback = { button: '按钮', text: '文本组件', input: '输入框', image: '图片组件', list: '列表', spacer: '间隔占位', group: '布局容器' }[node.kind]
  return TYPE_LABELS[simpleTag(node.tag).toLowerCase()] ?? fallback ?? '自定义组件'
}

function identifierTokens(value: string) {
  return value
    .replace(/^@\+?id\//, '')
    .replace(/^id\//, '')
    .replace(/([a-z0-9])([A-Z])/g, '$1 $2')
    .replace(/([A-Z]+)([A-Z][a-z])/g, '$1 $2')
    .split(/[_\-.\s]+/)
    .map((token) => token.toLowerCase())
    .filter(Boolean)
}

function translatedIdentifier(value: string) {
  return identifierTokens(value).map((token) => TOKEN_LABELS[token]).filter(Boolean).join('')
}

function visibleText(node: SourcePreviewNode) {
  const text = node.style.text.replace(/\s+/g, ' ').trim()
  if (!text || text.startsWith('@') || text.length > 28) return ''
  return text
}

export function getSourcePreviewNodeLabels(node: SourcePreviewNode): SourcePreviewNodeLabels {
  const type = componentType(node)
  const primary = visibleText(node) || translatedIdentifier(node.resourceId ?? node.name) || type
  const rawId = node.resourceId ?? (node.name !== simpleTag(node.tag) ? node.name : '')
  const tooltip = [
    `中文名称：${primary}`,
    `组件类型：${type}`,
    rawId ? `原始标识：${rawId}` : '',
    `Android 类型：${node.tag}`,
  ].filter(Boolean).join('\n')
  return { primary, type, tooltip }
}

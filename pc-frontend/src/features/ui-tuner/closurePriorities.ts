export type UiTunerClosureStage =
  | 'capture'
  | 'clarity'
  | 'selection'
  | 'codex'
  | 'standard'
  | 'source'
  | 'patch'
  | 'review'
  | 'persist'
  | 'validate'
  | 'rollout'

export interface UiTunerClosurePriority {
  id: `P${number}`
  priority: number
  stage: UiTunerClosureStage
  title: string
  outcome: string
  automationTarget: string
  required: boolean
}

interface PriorityGroup {
  stage: UiTunerClosureStage
  start: number
  outcome: string
  automationTarget: string
  titles: string[]
}

export const UI_TUNER_STAGE_LABELS: Record<UiTunerClosureStage, string> = {
  capture: '真机采集',
  clarity: '画面清晰',
  selection: '元素选择',
  codex: 'Codex 对话',
  standard: '标准配置',
  source: '源码绑定',
  patch: '修改生成',
  review: '设计审查',
  persist: '标准沉淀',
  validate: '验收验证',
  rollout: '发布复用',
}

const PRIORITY_GROUPS: PriorityGroup[] = [
  {
    stage: 'capture',
    start: 0,
    outcome: '稳定拿到当前手机画面、XML、截图和 APK 包上下文。',
    automationTarget: 'adb_snapshot',
    titles: [
      '发现有线和无线 ADB 设备',
      '展示当前连接状态和设备来源',
      '采集截图与 XML 到同一快照',
      '记录 package 和 activity',
      '记录截图尺寸和 XML 节点数',
      '采集失败时保留上一帧可编辑画布',
    ],
  },
  {
    stage: 'clarity',
    start: 6,
    outcome: '把重叠 XML 层级过滤成开发者可判断的产品画面。',
    automationTarget: 'layer_filter',
    titles: [
      '默认过滤结构容器和重复边界',
      '提供产品模式和全部 XML 模式',
      '按源码映射和交互属性筛选',
      '隐藏非目标包节点',
      '锁定参考层避免误拖动',
      '保留调试入口查看被隐藏原因',
    ],
  },
  {
    stage: 'selection',
    start: 12,
    outcome: '用户点击任意可见节点后，系统能给出稳定身份和置信度。',
    automationTarget: 'element_identity',
    titles: [
      '选中节点高亮并同步图层',
      '展示 resourceId 和 xpath',
      '展示源码文件和 token',
      '展示 bounds 与画布坐标换算',
      '提示源码绑定置信度',
      '允许用户把节点标记为标准草案',
    ],
  },
  {
    stage: 'codex',
    start: 18,
    outcome: '右侧面板能把当前选中节点转成 Codex 可执行任务上下文。',
    automationTarget: 'codex_context_pack',
    titles: [
      '生成选中元素上下文 JSON',
      '生成 Codex 任务提示',
      '把用户意图合入任务提示',
      '明确不要只改截图坐标',
      '列出需要回写的 Android 目标',
      '为后续 sidecar 终端预留任务载荷',
    ],
  },
  {
    stage: 'standard',
    start: 24,
    outcome: '所有设计判断保存成可配置标准，而不是散落在说明文里。',
    automationTarget: 'ui_standard_config',
    titles: [
      '区分 design token 和组件标准',
      '区分页面覆盖和本机草稿',
      '记录颜色字号圆角间距 token',
      '记录组件 role/variant/reuseKey',
      '导出标准草案 JSON',
      '把 P0-P65 本身纳入配置',
    ],
  },
  {
    stage: 'source',
    start: 30,
    outcome: '从运行时节点追到 Android 源码中的 layout、values 或 Kotlin 调用点。',
    automationTarget: 'source_binding',
    titles: [
      '通过 resourceId 反查 res/layout',
      '通过源码 token 定位 values',
      '通过 activity 缩小页面范围',
      '记录低置信度节点的人工确认项',
      '把可复用改动优先放入 token',
      '把一次性改动限制到 screen override',
    ],
  },
  {
    stage: 'patch',
    start: 36,
    outcome: '把画布调整转成可审查、可回滚的源码修改建议。',
    automationTarget: 'android_patch_plan',
    titles: [
      '生成 XML/value 修改计划',
      '保留修改前后 style diff',
      '标记会影响多页面的 token',
      '标记只影响当前页面的 override',
      '要求 Codex 先读相关源码再改',
      '要求输出验证命令和结果',
    ],
  },
  {
    stage: 'review',
    start: 42,
    outcome: '把 UI 美观标准变成可检查规则，减少主观争议。',
    automationTarget: 'design_review',
    titles: [
      '检查字号层级是否合理',
      '检查间距是否落在 token 梯度',
      '检查颜色对比和品牌一致性',
      '检查按钮和卡片是否复用组件',
      '检查文本是否溢出容器',
      '检查隐藏层级是否仍可追溯',
    ],
  },
  {
    stage: 'persist',
    start: 48,
    outcome: '调整结果可以复用到下一次捕获、下一页和下一版 APK。',
    automationTarget: 'standard_store',
    titles: [
      '保存 tokens.json',
      '保存 components.json',
      '保存 screens 页面覆盖',
      '保存选中节点到源码绑定',
      '保存标准版本和变更说明',
      '支持从标准重新加载画布',
    ],
  },
  {
    stage: 'validate',
    start: 54,
    outcome: '源码修改后能重新安装或热更新，并用真机截图验证效果。',
    automationTarget: 'device_validation',
    titles: [
      '运行 Android 构建检查',
      '安装或更新 APK',
      '重新采集同一页面截图',
      '对比修改前后 XML 节点',
      '对比选中元素最终 bounds',
      '生成验收报告和残留问题',
    ],
  },
  {
    stage: 'rollout',
    start: 60,
    outcome: '形成团队可持续使用的微调画布工作流。',
    automationTarget: 'release_loop',
    titles: [
      '把标准随项目版本提交',
      '把 Codex 任务链接到项目频道',
      '把 sidecar 输出回写到右侧面板',
      '把发布状态写回标准版本',
      '把失败案例沉淀为过滤规则',
      '把通过验收的标准作为下一次默认值',
    ],
  },
]

export const UI_TUNER_CLOSURE_PRIORITIES: UiTunerClosurePriority[] = PRIORITY_GROUPS.flatMap((group) => (
  group.titles.map((title, index) => {
    const priority = group.start + index
    return {
      id: `P${priority}` as `P${number}`,
      priority,
      stage: group.stage,
      title,
      outcome: group.outcome,
      automationTarget: group.automationTarget,
      required: priority < 24,
    }
  })
))

export function summarizeClosurePriorities() {
  return PRIORITY_GROUPS.map((group) => ({
    stage: group.stage,
    label: UI_TUNER_STAGE_LABELS[group.stage],
    range: `P${group.start}-P${group.start + group.titles.length - 1}`,
    outcome: group.outcome,
    automationTarget: group.automationTarget,
    count: group.titles.length,
  }))
}

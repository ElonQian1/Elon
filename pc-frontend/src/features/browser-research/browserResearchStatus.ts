import type { ResearchSession } from './types'

export function collecting(session: ResearchSession, now: number): boolean {
  return session.active && session.phase === 'observing' && session.expires_at_ms > now
}

export function phaseLabel(value: string): string {
  return ({ observing: '采集中', paused: '已暂停', expired: '已过期', opening: '正在打开',
    resuming: '正在恢复连接', loading: '页面正在加载', login: '等待官网登录', restored: '已恢复资料',
    closed: '已关闭', unavailable: '宿主不可用', host_unavailable: '采集连接失败',
    scope_changed: '站点配置已变更', ready: '已就绪' } as Record<string, string>)[value] ?? '状态待确认'
}

export function hostStageLabel(value: string): string {
  return ({ native_dispatch: '准备窗口', native_create: '创建浏览器', native_attached: '已连接浏览器',
    page_enable: '连接页面事件', frame_tree: '确认主页面', runtime_reset: '重置页面上下文',
    runtime_enable: '连接页面上下文', debugger_reset: '重置脚本观察', network_enable: '连接请求观察',
    debugger_safe_mode: '准备脚本观察', debugger_enable: '连接脚本资源' } as Record<string, string>)[value] ?? '等待连接回执'
}

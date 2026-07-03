export interface FullAccessGrant {
  project_id: string
  workspace_path: string
  granted_at_ms?: number
}

export type WorkspaceAccessLoadState = 'loading' | 'ready' | 'offline' | 'error'

export function workspaceAccessStatus({
  loadState,
  matchingGrant,
  fullAccessRequired,
  localNodeIsBound,
  hasBoundNode,
}: {
  loadState: WorkspaceAccessLoadState
  matchingGrant?: FullAccessGrant
  fullAccessRequired: boolean
  localNodeIsBound: boolean
  hasBoundNode: boolean
}) {
  if (loadState === 'loading') return { tone: 'neutral', label: '检查中', summary: '正在读取这台电脑的节点和授权状态。' }
  if (loadState === 'offline' || loadState === 'error') {
    return { tone: 'danger', label: '本机未就绪', summary: '请先启动一龙 Win 端并登录当前账号，然后刷新状态。' }
  }
  if (!hasBoundNode || !localNodeIsBound) {
    return { tone: 'danger', label: '尚未绑定本机', summary: '选择项目目录后，工作台会把当前项目绑定到这台电脑并写入本机授权。' }
  }
  if (!fullAccessRequired) {
    return { tone: 'success', label: '无需完全访问', summary: '当前为项目目录写入模式，不需要额外的本机完全访问授权。' }
  }
  if (matchingGrant) {
    return { tone: 'success', label: '已授权', summary: '这台电脑已确认该项目目录，可以重新发送 AI 开发任务。' }
  }
  return { tone: 'danger', label: '等待本机确认', summary: '云端已启用完全访问，但这台电脑还没有确认当前项目目录。请使用下方按钮授权。' }
}

export function runtimePermissionLabel(value?: string | null) {
  if (value === 'danger_full_access') return '完整本机命令行'
  if (value === 'full_access') return '完全访问'
  return '项目目录写入'
}

export function sameWorkspacePath(left: string, right: string) {
  const normalize = (value: string) => {
    const path = value.trim().replace(/\\/g, '/').replace(/\/+$/, '')
    return /^[a-z]:\//i.test(path) ? path.toLowerCase() : path
  }
  return !!left && !!right && normalize(left) === normalize(right)
}

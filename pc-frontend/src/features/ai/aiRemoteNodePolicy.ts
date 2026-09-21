export interface RemoteNodeInfo {
  node_id?: string
  agent_id?: string
  display_name?: string
  device_name?: string
  owner_user_id?: string
  online?: boolean
  ai_cli_ready?: boolean
  route_a_ready?: boolean
  allowed_clis?: string[]
}
export function remoteNodeId(node: RemoteNodeInfo) {
  return String(node.node_id ?? node.agent_id ?? '').trim()
}
export function remoteNodeName(node: RemoteNodeInfo) {
  const id = remoteNodeId(node)
  return String(node.display_name ?? node.device_name ?? id.slice(0, 8) ?? '远程节点').trim()
}

export function remoteNodeHasCli(node: RemoteNodeInfo) {
  return !!node.online && (
    node.ai_cli_ready === true
    || node.route_a_ready === true
    || (node.allowed_clis?.length ?? 0) > 0
  )
}

export function pickRemoteCliNode(
  nodes: RemoteNodeInfo[],
  userId?: string,
  preferredNodeId?: string | null,
  excludedNodeIds: string[] = [],
) {
  const excluded = new Set(excludedNodeIds.filter(Boolean))
  const ready = nodes.filter((node) => {
    const id = remoteNodeId(node)
    return id && !excluded.has(id) && remoteNodeHasCli(node)
  })
  if (preferredNodeId) {
    const preferred = ready.find((node) => remoteNodeId(node) === preferredNodeId)
    if (preferred) return preferred
  }
  return ready.find((node) => node.owner_user_id && node.owner_user_id !== userId) ?? ready[0] ?? null
}

export function shouldRetryRemoteNodeExec(result: { output?: string; error?: string; exit_ok?: boolean }) {
  if ((result.output ?? '').trim()) return false
  const error = result.error ?? ''
  return result.exit_ok === false && (
    error.includes('指定的节点未在线')
    || error.includes('没有确认接收')
    || error.includes('没有返回任何 CLI 输出')
    || error.includes('连接假在线')
    || error.includes('通道在确认接收')
    || error.includes('节点连接已关闭')
    || error.includes('执行超时')
  )
}

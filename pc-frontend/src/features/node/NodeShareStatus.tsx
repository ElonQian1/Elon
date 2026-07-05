import type { NodeSummary } from './types'
import styles from './NodePage.module.css'

export function publicDevHandshakeText(node: NodeSummary): string {
  if (node.public_dev_handshake_ready) return '就绪'
  const status = node.public_dev_handshake_status ?? ''
  const labels: Record<string, string> = {
    sharing_disabled: '未开放',
    offline: '节点离线',
    waiting_for_handshake: '等待握手',
    version_reconnected_waiting_capabilities: '等待能力刷新',
    no_allowed_cli: 'CLI 不匹配',
    runtime_not_ready: '运行时未就绪',
    ready: '就绪',
  }
  return labels[status] ?? (status || '未知')
}

function readyText(value?: boolean): string {
  return value ? '就绪' : '未就绪'
}

function connectedText(value?: boolean): string {
  return value ? '已连接' : '未连接'
}

export default function NodeShareStatus({ node }: { node: NodeSummary }) {
  return (
    <div className={styles.shareStatus}>
      <div>
        <strong>远程开发共享</strong>
        <span>{publicDevHandshakeText(node)}</span>
      </div>
      <p>
        权限 {node.public_dev_permission_level ?? 'project_write'} · 最近握手{' '}
        {node.last_handshake_at ?? '等待节点重连'} · 节点上报 CLI{' '}
        {(node.last_handshake_allowed_clis ?? node.allowed_clis ?? []).join(' / ') || '未上报'}
      </p>
      <p>
        调试 注册 {connectedText(node.registry_online ?? node.online)} · CLI 通道{' '}
        {connectedText(node.cli_connected)} · Route A {readyText(node.route_a_ready)} · API{' '}
        {readyText(node.api_runtime_ready)} · Server {readyText(node.server_runtime_ready)}
      </p>
    </div>
  )
}

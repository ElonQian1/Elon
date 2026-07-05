import { lifecycleView } from './nodeLifecycle'
import type { LocalNodeStatus } from './types'

export type DiagnosticTone = 'ok' | 'warning' | 'danger' | 'neutral'

export interface DiagnosticItem {
  key: string
  title: string
  detail: string
  badge: string
  tone: DiagnosticTone
}

export interface LocalDiagnosticView {
  title: string
  detail: string
  tone: DiagnosticTone
  items: DiagnosticItem[]
}

export function localDiagnosticView(status: LocalNodeStatus): LocalDiagnosticView {
  const lifecycle = lifecycleView(status.lifecycle, {
    connected: status.connected,
    loggedIn: status.logged_in,
    lastEvent: status.last_event,
  })
  const cloudHost = hostLabel(status.cloud_http_url || status.cloud_url || '')
  const cloudDetail = status.connected
    ? `已直连云端${cloudHost ? ` ${cloudHost}` : ''}。`
    : `本机接口可用，但云端${cloudHost ? ` ${cloudHost}` : ''}暂时未连接。`
  const networkDetail = networkSummary(status)
  const routerDetail = routerSummary(status)
  const items: DiagnosticItem[] = [
    {
      key: 'local',
      title: '本机程序',
      detail: `HTTP 管理接口已响应，版本 ${status.version || '未知'}。`,
      badge: '可访问',
      tone: 'ok',
    },
    {
      key: 'cloud',
      title: '云端连接',
      detail: cloudDetail,
      badge: status.connected ? '已连接' : '未连接',
      tone: status.connected ? 'ok' : 'warning',
    },
    {
      key: 'lifecycle',
      title: lifecycle.title,
      detail: lifecycle.detail,
      badge: lifecycle.badge,
      tone: lifecycle.tone,
    },
    {
      key: 'network',
      title: '网络路由',
      detail: networkDetail,
      badge: networkBadge(status),
      tone: networkTone(status),
    },
    {
      key: 'router',
      title: '下载/代理',
      detail: routerDetail,
      badge: routerBadge(status),
      tone: routerTone(status),
    },
  ]
  const worstTone = worst(items.map((item) => item.tone))
  return {
    title: headlineFor(worstTone, status),
    detail: detailFor(worstTone, status),
    tone: worstTone,
    items,
  }
}

export function buildLocalDiagnosticCopy(status: LocalNodeStatus): string {
  const snapshot = {
    schema: 'elon.local_node_diagnostics.v1',
    generatedAt: new Date().toISOString(),
    localApiReachable: true,
    version: status.version ?? null,
    loggedIn: status.logged_in ?? null,
    connected: status.connected ?? null,
    deviceName: status.device_name ?? null,
    agentId: shortId(status.agent_id),
    cloudUrl: status.cloud_url ?? null,
    cloudHttpUrl: status.cloud_http_url ?? null,
    lastEvent: status.last_event ?? null,
    lifecycle: status.lifecycle ?? null,
    cloudNetwork: status.cloud_network ?? null,
    downloadRouter: status.download_router ?? null,
    cliProbe: status.cli_probe ?? null,
    codexCli: status.codex_cli
      ? {
          status: status.codex_cli.status ?? null,
          installed: status.codex_cli.installed ?? null,
          runnable: status.codex_cli.runnable ?? null,
          loggedIn: status.codex_cli.logged_in ?? null,
          detail: status.codex_cli.detail ?? null,
          diagnosis: status.codex_cli.diagnosis ?? null,
          fixHint: status.codex_cli.fix_hint ?? null,
        }
      : null,
  }
  return JSON.stringify(snapshot, null, 2)
}

function networkSummary(status: LocalNodeStatus): string {
  const network = status.cloud_network
  if (!network) return '暂无云端网络诊断字段。'
  const httpMode = network.cloudHttpMode || '未知 HTTP 模式'
  const wsMode = network.cloudWsMode || '未知 WebSocket 模式'
  const proxy = network.proxyDefault || '未知代理策略'
  const hosts = network.cloudHostsNoProxy?.filter(Boolean).join('、')
  return `${httpMode}；${wsMode}；默认策略 ${proxy}${hosts ? `；直连主机 ${hosts}` : ''}。`
}

function routerSummary(status: LocalNodeStatus): string {
  const profile = status.download_router?.profile
  if (!profile) return '下载路由配置暂未返回。'
  const mode = profile.mode || 'auto'
  const enabled = profile.enabled === false ? '关闭' : '启用'
  const failOpen = profile.failOpen === false ? '严格' : '失败放行'
  return `下载路由 ${enabled}，模式 ${mode}，策略 ${failOpen}。`
}

function networkBadge(status: LocalNodeStatus): string {
  if (!status.cloud_network) return '未知'
  if (status.cloud_network.proxyDefault === 'off_for_elon_cloud') return '云端直连'
  return '需确认'
}

function networkTone(status: LocalNodeStatus): DiagnosticTone {
  if (!status.cloud_network) return 'neutral'
  return status.cloud_network.proxyDefault === 'off_for_elon_cloud' ? 'ok' : 'warning'
}

function routerBadge(status: LocalNodeStatus): string {
  const profile = status.download_router?.profile
  if (!profile) return '未知'
  if (profile.enabled === false || profile.mode === 'off') return '关闭'
  return profile.mode || 'auto'
}

function routerTone(status: LocalNodeStatus): DiagnosticTone {
  const profile = status.download_router?.profile
  if (!profile) return 'neutral'
  return profile.enabled === false || profile.mode === 'off' ? 'warning' : 'ok'
}

function headlineFor(tone: DiagnosticTone, status: LocalNodeStatus): string {
  if (tone === 'danger') return '本机节点需要处理'
  if (!status.connected) return '本机正常，云端连接待恢复'
  if (tone === 'warning') return '本机可用，有项目需要确认'
  return '本机节点运行正常'
}

function detailFor(tone: DiagnosticTone, status: LocalNodeStatus): string {
  if (tone === 'danger') return '生命周期或本机运行状态提示异常，建议重启 Win 端并导出诊断。'
  if (!status.connected) return '本机页面和管理接口可用，问题更可能在云端网络、代理、防火墙或账号连接。'
  if (tone === 'warning') return '可以继续使用；建议按下方项目排查网络路由、下载代理或生命周期提示。'
  return '本机 HTTP、云端连接、生命周期和网络直连策略都处于可用状态。'
}

function worst(tones: DiagnosticTone[]): DiagnosticTone {
  if (tones.includes('danger')) return 'danger'
  if (tones.includes('warning')) return 'warning'
  if (tones.includes('neutral')) return 'neutral'
  return 'ok'
}

function hostLabel(raw: string): string {
  try {
    return new URL(raw).host
  } catch {
    return raw
  }
}

function shortId(value: string | undefined): string | null {
  if (!value) return null
  if (value.length <= 12) return value
  return `${value.slice(0, 6)}...${value.slice(-4)}`
}

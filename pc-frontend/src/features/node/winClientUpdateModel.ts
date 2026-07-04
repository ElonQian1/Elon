import type { NodeAgentVersion } from './types'

export type WinClientUpdateKind = 'unknown' | 'current' | 'update'

export interface WinClientUpdateState {
  kind: WinClientUpdateKind
  title: string
  detail: string
  localLabel: string
  latestLabel: string
}

export function buildWinClientUpdateState(
  localVersion: string | null | undefined,
  localGitSha: string | null | undefined,
  latest: NodeAgentVersion | null | undefined,
): WinClientUpdateState {
  const local = cleanText(localVersion)
  const localSha = cleanText(localGitSha)
  const latestVersion = cleanText(latest?.version)
  const latestSha = cleanText(latest?.gitSha)
  const localLabel = local || (localSha ? shortSha(localSha) : '未知')
  const latestLabel = latestVersion || (latestSha ? shortSha(latestSha) : '未知')

  if (!latestVersion && !latestSha) {
    return {
      kind: 'unknown',
      title: '暂时无法读取服务器最新版本',
      detail: '网页端稍后会自动重试；也可以直接下载最新 Win 端覆盖安装。',
      localLabel,
      latestLabel,
    }
  }
  if (!local && !localSha) {
    return {
      kind: 'unknown',
      title: '本机版本未知',
      detail: '旧版 Win 端可能没有上报完整版本；建议检查更新以收敛到最新客户端。',
      localLabel,
      latestLabel,
    }
  }

  const versionCompare = compareVersionText(local, latestVersion)
  if (versionCompare < 0 || (versionCompare === 0 && localSha && latestSha && localSha !== latestSha)) {
    return {
      kind: 'update',
      title: 'Win 端有新版本',
      detail: `本机 ${localLabel}，服务器 ${latestLabel}。更新会下载最新完整客户端包并重启本机节点。`,
      localLabel,
      latestLabel,
    }
  }

  return {
    kind: 'current',
    title: 'Win 端已是最新',
    detail: `本机 ${localLabel}，服务器 ${latestLabel}。`,
    localLabel,
    latestLabel,
  }
}

export function compareVersionText(left: string, right: string): number {
  const a = versionParts(left)
  const b = versionParts(right)
  if (!a.length || !b.length) return 0
  const len = Math.max(a.length, b.length)
  for (let index = 0; index < len; index += 1) {
    const delta = (a[index] ?? 0) - (b[index] ?? 0)
    if (delta !== 0) return delta > 0 ? 1 : -1
  }
  return 0
}

export function shortSha(value: string): string {
  const text = cleanText(value)
  if (text.length <= 10) return text
  return text.slice(0, 8)
}

function versionParts(value: string): number[] {
  return cleanText(value)
    .replace(/^v/i, '')
    .split(/[^\d]+/)
    .filter(Boolean)
    .map((part) => Number.parseInt(part, 10))
    .filter((part) => Number.isFinite(part))
}

function cleanText(value: string | null | undefined): string {
  return String(value ?? '').trim()
}

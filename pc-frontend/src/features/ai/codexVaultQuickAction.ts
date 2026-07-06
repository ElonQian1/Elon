import { nodeApi, probeLocalNode } from '../node/localNodeApi'
import type {
  CodexVaultAuthInspection,
  CodexVaultCloudStatus,
  CodexVaultStatusResponse,
  LocalNodeStatus,
} from '../node/types'

function includesAny(text: string, needles: string[]): boolean {
  return needles.some((needle) => text.includes(needle))
}

export function isCodexVaultBackupIntent(text: string): boolean {
  const compact = text.toLowerCase().replace(/\s+/g, '')
  const mentionsAuth = includesAny(compact, [
    'codex账号',
    'codex账户',
    'codex登录',
    'codex凭据',
    'codexpro',
  ])
  const mentionsVault = includesAny(compact, ['保险箱', 'vault', '云端保险', '云端保存', '云端备份'])
  const mentionsAccountSharing = includesAny(compact, [
    '分享codex账号',
    '共享codex账号',
    '分享codex账户',
    '共享codex账户',
  ])
  const wantsBackup = includesAny(compact, [
    '备份',
    '保存',
    '分享',
    '共享',
    '上传',
    '存到',
    '存进',
    '存入',
    'backup',
    'save',
  ])
  const asksOnly = includesAny(compact, ['是否', '有没有', '了吗', '查看', '查询', '状态', '检查', '检测', '怎么', '如何', '哪里'])
  const explicitAction = includesAny(compact, [
    '帮我',
    '请',
    '一键',
    '现在',
    '马上',
    '备份到',
    '保存到',
    '分享到',
    '共享到',
    '上传到',
    '存到',
    '存进',
    '存入',
    '备份本机',
    '保存本机',
    '上传本机',
    '分享本机',
    '共享本机',
  ])
  return mentionsAuth && (mentionsVault || mentionsAccountSharing) && wantsBackup && (!asksOnly || explicitAction)
}

function formatVaultTime(value?: string | null): string {
  if (!value) return '刚刚'
  const date = new Date(value)
  if (Number.isNaN(date.getTime())) return value
  return date.toLocaleString('zh-CN')
}

function normalizeError(error: unknown): string {
  const message = (error as { message?: string })?.message?.trim()
  if (!message) return '操作失败'
  if (/abort|timeout|failed to fetch|network/i.test(message)) {
    return '没有检测到本机 Win 端。请先打开或安装一龙 Win 端，再重试保存 Codex 账号。'
  }
  return message
}

function assertUsableDefaultAuth(auth?: CodexVaultAuthInspection): void {
  if (!auth?.present) {
    throw new Error('没有检测到这台电脑的 Codex 账号。请先在这台电脑用 Codex / ChatGPT 登录，然后再保存。')
  }
  if (auth.problem) throw new Error(auth.problem)
  if (auth.auth_mode === 'api_key') {
    throw new Error('当前是 OpenAI API Key 模式；账号保险箱只保存 ChatGPT / Pro 登录态。')
  }
  if (auth.auth_mode && auth.auth_mode !== 'chatgpt') {
    throw new Error(`当前 Codex 账号模式是 ${auth.auth_mode}，不是 ChatGPT / Pro 登录态。`)
  }
  if (!auth.has_refresh_token) {
    throw new Error('当前 Codex 账号缺少可续期登录凭据。请重新用 Codex / ChatGPT 登录后再保存。')
  }
}

function latestCredentialVersion(vault?: CodexVaultCloudStatus): number | null {
  if (typeof vault?.credential_version === 'number') return vault.credential_version
  const versions = (vault?.slots ?? [])
    .map((slot) => slot.credential_version)
    .filter((value): value is number => typeof value === 'number' && Number.isFinite(value))
  return versions.length ? Math.max(...versions) : null
}

function buildSuccessMessage(data: CodexVaultStatusResponse, local: LocalNodeStatus): string {
  const vault = data.cloud?.vault
  const count = vault?.available_count ?? vault?.slots?.length ?? (vault?.bound ? 1 : 0)
  const version = latestCredentialVersion(vault)
  const sourceDevice = vault?.source_device || local.device_name || '这台电脑'
  const savedAt = formatVaultTime(vault?.last_backup_at)
  const cloudLine = count > 1 ? `云端：已保存 ${count} 个账号槽位` : '云端：已绑定当前账号保险箱'
  const versionLine = version ? ` · v${version}` : ''

  return [
    '已把这台电脑的 Codex 账号加密保存到云端账号保险箱。',
    '',
    `- 本机：${sourceDevice}`,
    `- ${cloudLine}${versionLine}`,
    `- 时间：${savedAt}`,
    '',
    'Codex 账号凭据不会展示给网页或聊天模型；它只在本机节点内读取并加密保存。',
  ].join('\n')
}

export async function runCodexVaultBackupFromAiChat(adminUrl: string): Promise<string> {
  let local: LocalNodeStatus
  try {
    local = await probeLocalNode(adminUrl) as LocalNodeStatus
  } catch (error) {
    throw new Error(normalizeError(error))
  }

  if (!local.logged_in) {
    throw new Error('本机 Win 端还没有绑定当前一龙账号。请先到 /pc/node 用当前账号注册节点，然后再保存 Codex 账号。')
  }
  if (local.connected === false) {
    throw new Error('本机 Win 端还没有连上云端。请保持 Win 端在线后再保存 Codex 账号。')
  }

  let status: CodexVaultStatusResponse | null = null
  try {
    status = await nodeApi<CodexVaultStatusResponse>(adminUrl, '/api/codex-vault/status', {}, 12000)
  } catch {
    status = null
  }
  const defaultAuth = status?.local?.default_auth ?? local.codex_vault?.default_auth
  assertUsableDefaultAuth(defaultAuth)
  if (status?.cloud?.vault?.configured === false) {
    throw new Error('服务器还没有配置 Codex 账号保险箱主密钥，暂时不能云端保存。')
  }
  if (status?.cloud?.error) throw new Error(status.cloud.error)

  let backup: CodexVaultStatusResponse
  try {
    backup = await nodeApi<CodexVaultStatusResponse>(
      adminUrl,
      '/api/codex-vault/backup',
      { method: 'POST', body: JSON.stringify({}) },
      30000,
    )
  } catch (error) {
    throw new Error(normalizeError(error))
  }
  if (backup.ok === false) {
    throw new Error(backup.error || backup.message || 'Codex 账号保存失败。')
  }
  if (backup.cloud?.error) throw new Error(backup.cloud.error)
  if (backup.cloud?.vault?.configured === false) {
    throw new Error('服务器还没有配置 Codex 账号保险箱主密钥，暂时不能云端保存。')
  }
  return buildSuccessMessage(backup, local)
}

export type CodexVaultConsentAction =
  | 'backup'
  | 'restore'
  | 'restore_shared'
  | 'clear_local'
  | 'delete_cloud'

const confirmations: Record<CodexVaultConsentAction, string> = {
  backup: 'BACKUP_CODEX_VAULT',
  restore: 'RESTORE_CODEX_VAULT',
  restore_shared: 'RESTORE_SHARED_CODEX_VAULT',
  clear_local: 'CLEAR_MANAGED_CODEX_HOME',
  delete_cloud: 'DELETE_CLOUD_CODEX_VAULT',
}

const prompts: Record<CodexVaultConsentAction, string> = {
  backup: '确认把本机 Codex 登录凭据加密备份到你的云端账号保险箱？',
  restore: '确认从你的云端账号保险箱恢复 Codex 登录凭据到本机托管目录？',
  restore_shared: '确认使用已获授权的共享 Codex 账号，并在本机创建临时登录目录？',
  clear_local: '确认清理本机所有由一龙托管的临时 Codex 登录目录？',
  delete_cloud: '确认永久删除云端账号保险箱中的 Codex 登录凭据？此操作不可撤销。',
}

function requestId(): string {
  if (typeof crypto !== 'undefined' && typeof crypto.randomUUID === 'function') {
    return `vault:${crypto.randomUUID()}`
  }
  return `vault:${Date.now()}:${Math.random().toString(36).slice(2)}`
}

export function codexVaultConsentBody(action: CodexVaultConsentAction, purpose?: string) {
  return {
    request_id: requestId(),
    explicit_consent: true,
    confirmation: confirmations[action],
    ...(purpose ? { purpose } : {}),
  }
}

export function confirmCodexVaultAction(
  action: CodexVaultConsentAction,
  purpose?: string,
): ReturnType<typeof codexVaultConsentBody> | null {
  if (!window.confirm(prompts[action])) return null
  return codexVaultConsentBody(action, purpose)
}

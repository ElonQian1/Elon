import { nodeApi } from './localNodeApi'
import type { CodexVaultStatusResponse } from './types'

export interface CodexVaultEmergencyActions {
  onEmergencyRestore: (providerUserId: string) => Promise<void>
  onCreateEmergencyGrant: (consumerAccount: string) => Promise<void>
  onRevokeEmergencyGrant: (grantId: string) => Promise<void>
}

export function createCodexVaultEmergencyActions({
  adminUrl,
  setVaultBusy,
  setCodexBusy,
  setResult,
  setError,
  setVaultStatus,
  refreshStatus,
  loadCodexVaultStatus,
}: {
  adminUrl: string
  setVaultBusy: (busy: boolean) => void
  setCodexBusy: (busy: boolean) => void
  setResult: (message: string) => void
  setError: (message: string) => void
  setVaultStatus: (update: CodexVaultStatusResponse | null | ((prev: CodexVaultStatusResponse | null) => CodexVaultStatusResponse | null)) => void
  refreshStatus: (quiet?: boolean) => Promise<void>
  loadCodexVaultStatus: (quiet?: boolean) => Promise<void>
}): CodexVaultEmergencyActions {
  return {
    async onCreateEmergencyGrant(consumerAccount: string) {
      setVaultBusy(true); setResult('正在保存机器人授权共享…'); setError('')
      try {
        const data = await nodeApi<{ message?: string }>(
          adminUrl,
          '/api/codex-vault/sharing/grants',
          {
            method: 'POST',
            body: JSON.stringify({
              consumer_account: consumerAccount,
              purpose: 'robot_codex_vault_shared_access',
              max_lease_seconds: 900,
            }),
          },
          20000,
        )
        await loadCodexVaultStatus(true)
        setResult(data.message || '机器人授权共享已保存。')
      } catch (err) {
        setError((err as Error).message)
      } finally {
        setVaultBusy(false)
      }
    },
    async onRevokeEmergencyGrant(grantId: string) {
      setVaultBusy(true); setResult('正在撤销机器人授权共享…'); setError('')
      try {
        const data = await nodeApi<{ message?: string }>(
          adminUrl,
          `/api/codex-vault/sharing/grants/${encodeURIComponent(grantId)}`,
          { method: 'DELETE' },
          20000,
        )
        await loadCodexVaultStatus(true)
        setResult(data.message || '机器人授权共享已撤销。')
      } catch (err) {
        setError((err as Error).message)
      } finally {
        setVaultBusy(false)
      }
    },
    async onEmergencyRestore(providerUserId: string) {
      setVaultBusy(true); setCodexBusy(true); setResult('正在切换到授权机器人的共享 Codex Pro 会话…'); setError('')
      try {
        const data = await nodeApi<CodexVaultStatusResponse>(
          adminUrl,
          '/api/codex-vault/sharing/restore',
          {
            method: 'POST',
            body: JSON.stringify({
              provider_user_id: providerUserId,
              purpose: 'pc_web_robot_shared_codex_cli',
            }),
          },
          30000,
        )
        setVaultStatus((prev) => ({
          ...(prev ?? {}),
          ok: data.ok,
          local: data.local,
          message: data.message,
          cloud: prev?.cloud,
        }))
        await refreshStatus(true)
        await loadCodexVaultStatus(true)
        setResult(data.message || '已切换到授权机器人的共享 Codex Pro 会话。')
      } catch (err) {
        setError((err as Error).message)
      } finally {
        setVaultBusy(false)
        setCodexBusy(false)
      }
    },
  }
}

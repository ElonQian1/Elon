import { v4 as uuidv4 } from 'uuid'
import { useAuthStore } from '../../../store/auth'
import { cloudBaseUrl, isLocalWorkbench } from '../../../api/runtime'
import { getDesktopInvoke } from '../../shell/desktopShell'
import { postWinEvent } from '../../codex-control/codexControlApi'
import { safeNodeAdminUrl } from '../../../lib/utils'
import { nodeApi, probeLocalNode } from '../../node/localNodeApi'
import { socialRequest } from '../socialChatOperations'
import { getGroupAiTask, startGroupAi } from './groupAiStore'
import { GroupAiControlError, GroupAiControlModel, type GroupAiCommand } from './groupAiControlModel'

const base = '/api/codex-control/group-ai'
const worker = uuidv4()
let polling = false
let lastWorkerErrorAt = 0
const model = new GroupAiControlModel({
  owner: () => useAuthStore.getState().token ? useAuthStore.getState().user?.id || '' : '',
  uuid: uuidv4, read: socialRequest, task: getGroupAiTask, start: startGroupAi,
  async checkIdentity(owner) {
    const me = await socialRequest<{ user: { id: string } }>('/api/me')
    if (me.user?.id !== owner) throw new GroupAiControlError('account_changed')
    const local = await probeLocalNode(safeNodeAdminUrl()) as { logged_in?: boolean; owner_user_id?: string }
    if (local.logged_in && local.owner_user_id && local.owner_user_id !== owner) throw new GroupAiControlError('local_owner_conflict')
  },
})
let lastOwner = useAuthStore.getState().user?.id
useAuthStore.subscribe(state => {
  if (!state.token || state.user?.id !== lastOwner) model.invalidate()
  lastOwner = state.user?.id
})

export async function pollGroupAiCommands(backgroundWorker = false) {
  if (polling) return
  polling = true
  try {
    const url = safeNodeAdminUrl()
    const pending = await nodeApi<{ command_ids: string[] }>(url, base + '/pending')
    if (!pending.command_ids?.length) return
    if (!backgroundWorker) {
      // The local workbench origin has a different login store. Only the cloud-profile
      // worker may claim commands, including read-only discovery and task status.
      const invoke = getDesktopInvoke()
      try {
        if (invoke) await invoke('ensure_group_ai_worker', {
          cloudBaseUrl: isLocalWorkbench() ? cloudBaseUrl() : location.origin, nodeBaseUrl: url,
        })
      } catch (error) {
        if (Date.now() - lastWorkerErrorAt > 30_000) {
          lastWorkerErrorAt = Date.now()
          const code = String(error)
          void postWinEvent({ source: 'frontend', level: 'warn', kind: 'group.worker.unavailable',
            summary: '后台群聊执行页尚未就绪，命令未领取',
            fields: { code: /^(worker_|invalid_worker_|main_url_)[a-z_]+$/.test(code) ? code : 'worker_start_failed' },
          }).catch(() => {})
        }
      }
      return
    }
    for (const id of pending.command_ids ?? []) {
      let command: GroupAiCommand
      try {
        const claimed = await nodeApi<{ command: GroupAiCommand }>(url, `${base}/${encodeURIComponent(id)}/claim`, {
          method: 'POST', body: JSON.stringify({ worker_id: worker }),
        })
        command = claimed.command
      } catch { continue }
      let result: Record<string, unknown>
      try { result = await model.execute(command) }
      catch (error) { result = { schema: 'elon.win_group_ai_result.v1', ok: false,
        error_code: error instanceof GroupAiControlError ? error.message : 'group_command_failed' } }
      // Receipt retries never execute the business command again.
      for (let attempt = 0; attempt < 2; attempt++) {
        try {
          await nodeApi(url, `${base}/${encodeURIComponent(id)}/receipt`, { method: 'POST', body: JSON.stringify({ worker_id: worker, result }) })
          break
        } catch { if (!attempt) await new Promise(resolve => setTimeout(resolve, 400)) }
      }
    }
  } catch { /* Older nodes do not expose group commands. No user workflow is changed. */ }
  finally { polling = false }
}

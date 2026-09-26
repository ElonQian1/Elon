import { v4 as uuidv4 } from 'uuid'
import { useAuthStore } from '../../../store/auth'
import { safeNodeAdminUrl } from '../../../lib/utils'
import { nodeApi, probeLocalNode } from '../../node/localNodeApi'
import { socialRequest } from '../socialChatOperations'
import { getGroupAiTask, startGroupAi } from './groupAiStore'
import { GroupAiControlError, GroupAiControlModel, type GroupAiCommand } from './groupAiControlModel'

const base = '/api/codex-control/group-ai'
const worker = uuidv4()
let polling = false
const model = new GroupAiControlModel({
  owner: () => useAuthStore.getState().token ? useAuthStore.getState().user?.id || '' : '',
  uuid: uuidv4, read: socialRequest, task: getGroupAiTask, start: startGroupAi,
  async checkIdentity(owner) {
    const local = await probeLocalNode(safeNodeAdminUrl()) as { logged_in?: boolean; owner_user_id?: string }
    if (local.logged_in && local.owner_user_id && local.owner_user_id !== owner) throw new GroupAiControlError('local_owner_conflict')
  },
})
let lastOwner = useAuthStore.getState().user?.id
useAuthStore.subscribe(state => {
  if (!state.token || state.user?.id !== lastOwner) model.invalidate()
  lastOwner = state.user?.id
})

export async function pollGroupAiCommands() {
  if (polling) return
  polling = true
  try {
    const url = safeNodeAdminUrl()
    const pending = await nodeApi<{ command_ids: string[] }>(url, base + '/pending')
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

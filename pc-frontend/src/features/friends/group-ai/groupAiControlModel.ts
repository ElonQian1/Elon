import type { FriendGroup, SocialMessage } from '../socialMessageTypes'
import type { GroupAiInput, GroupAiTask } from './groupAiTask'
import type { GroupAiSources } from './groupAiContext'

export interface GroupAiCommand {
  command_id: string; action: 'groups' | 'messages' | 'start' | 'status' | 'resume' | 'cancel'
  owner_binding?: string | null; group_id?: string | null; task_id?: string | null
  question?: string | null; confirmed?: boolean; offset?: number
  message_ids?: string[]; message_revisions?: Record<string, number>
}
export interface GroupAiControlPort {
  owner(): string
  uuid(): string
  read<T>(path: string): Promise<T>
  start(input: GroupAiInput, operation: string): GroupAiTask
  task(): GroupAiTask | null
  checkIdentity(owner: string): Promise<void>
}
export class GroupAiControlError extends Error {}
const fail = (code: string): never => { throw new GroupAiControlError(code) }
const path = (group: string) => `/api/me/groups/${encodeURIComponent(group)}`
const recalled = (m: SocialMessage) => !!(m.recalled_at || m.recalledAt)

export class GroupAiControlModel {
  private binding = ''
  private owner = ''
  constructor(private port: GroupAiControlPort) {}
  invalidate() { this.owner = ''; this.binding = '' }
  async execute(c: GroupAiCommand): Promise<Record<string, unknown>> {
    const owner = this.port.owner()
    if (!owner) return fail('login_required')
    if (owner !== this.owner) { this.owner = owner; this.binding = this.port.uuid() }
    const binding = this.binding
    const check = () => {
      if (this.port.owner() !== owner || this.binding !== binding) fail('account_changed')
    }
    const read = async <T>(url: string) => { check(); const data = await this.port.read<T>(url); check(); return data }
    const output = (data: Record<string, unknown>) => { check(); return { schema: 'elon.win_group_ai_result.v1', ok: true, owner_binding: binding, ...data } }
    if (c.action !== 'groups' && c.owner_binding !== binding) return fail('owner_binding_stale')
    if (c.action === 'groups') { await this.port.checkIdentity(owner); check() }
    if (['groups', 'messages', 'start'].includes(c.action)) {
      const { groups } = await read<{ groups: FriendGroup[] }>('/api/me/groups')
      if (!Array.isArray(groups)) return fail('invalid_group_directory')
      const offset = c.offset ?? 0
      if (c.action === 'groups') return output({ groups: groups.slice(offset, offset + 20).map(g => ({ id: g.id, name: g.name.slice(0, 80) })), next_offset: offset + 20 < groups.length ? offset + 20 : null })
      const group = groups.find(g => g.id === c.group_id)
      if (!group) return fail('group_not_accessible')
      const { messages } = await read<{ messages: SocialMessage[] }>(path(group.id) + '/messages?limit=120&preserve_unread=true')
      if (!Array.isArray(messages)) return fail('invalid_message_list')
      if (c.action === 'messages') return output({ group_id: group.id, messages: [...messages].reverse().slice(offset, offset + 30).map(m => ({
        id: m.id, revision: m.revision ?? 1, preview: recalled(m) ? '' : m.content.slice(0, 240), created_at: m.created_at,
        recalled: recalled(m), attachment_count: m.attachments?.length ?? 0,
        has_image: !recalled(m) && !!m.attachments?.some(a => a.kind === 'image' || a.mime_type?.startsWith('image/')),
      })), next_offset: offset + 30 < messages.length ? offset + 30 : null })
      if (!c.confirmed || !c.question?.trim() || c.question.length > 2000 || !c.message_ids?.length || c.message_ids.length > 50) return fail('selection_confirmation_required')
      const ids = new Set(c.message_ids)
      if (ids.size !== c.message_ids.length) return fail('duplicate_selection')
      const selected = messages.filter(m => ids.has(m.id))
      if (selected.length !== ids.size || selected.some(m => recalled(m) || m.id.startsWith('tmp-') || (m.revision ?? 1) !== c.message_revisions?.[m.id])) return fail('selection_changed_or_unavailable')
      await this.port.checkIdentity(owner); check()
      const task = this.port.start({ owner, group: group.id, title: group.name, source: selected[0].id, provider: 'chatgpt', selection: {
        message_ids: selected.map(m => m.id), message_revisions: Object.fromEntries(selected.map(m => [m.id, m.revision ?? 1])),
        question: c.question.trim(), allow_continue: false,
      } }, c.command_id)
      return output(this.taskState(task))
    }
    const task = this.port.task()
    if (!task || task.operation !== c.task_id || task.input.owner !== owner) return fail('task_not_found')
    if (c.action === 'cancel') { await task.cancel(); return output(this.taskState(task)) }
    if (c.action === 'resume') {
      if (!task.dispatched || task.stopped) return fail('resume_requires_dispatched_task')
      void task.resume()
      return output(this.taskState(task))
    }
    let deliveryVerified = false, sourceVerified = false
    const result = task.request?.result_message_id
    if (task.progress.phase === 'completed' && result) {
      const { messages } = await read<{ messages: SocialMessage[] }>(path(task.input.group) + '/messages?limit=120&preserve_unread=true')
      const message = messages.find(m => m.id === result && !recalled(m))
      deliveryVerified = !!message?.content.trim() && (!task.answer || message.content === task.answer)
      const sources = await read<GroupAiSources>(path(task.input.group) + `/messages/${encodeURIComponent(result)}/ai-sources`)
      sourceVerified = sources.group_id === task.input.group && sources.message_id === result
        && sources.sources.length === task.input.selection.message_ids.length
        && sources.sources.every(m => task.input.selection.message_ids.includes(m.id))
    }
    return output({ ...this.taskState(task), delivery_verified: deliveryVerified, source_verified: sourceVerified })
  }
  private taskState(task: GroupAiTask): Record<string, unknown> {
    return { task_id: task.operation, task_found: true, group_id: task.input.group, request_id: task.request?.id ?? null,
      result_message_id: task.request?.result_message_id ?? null, phase: task.progress.phase, stage: task.stage,
      busy: task.progress.busy, dispatched: task.dispatched, has_answer: !!task.answer, answer_chars: task.answer.length,
      attachment_count: task.request?.attachments?.length ?? 0, source_count: task.input.selection.message_ids.length,
      message_count: task.observedMessageCount, error_code: task.lastReceiptCode || null }
  }
}

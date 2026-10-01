export interface Member { id: string; display_name: string; avatar_data_url?: string | null; role: 'owner' | 'admin' | 'member'; joined_at: string }
export interface Roster {
  group_id: string; name: string; total_count: number; matched_count: number; revision: number
  viewer_id: string; viewer_role: Member['role']; invitation_policy: string
  permissions: { invite: boolean; manage: boolean; owner: boolean }
  members: Member[]; next_cursor: string | null; pending_count: number
}
export interface Command { action: string; user_ids?: string[]; role?: string; invitation_policy?: string; invitation_id?: string }
export interface Receipt { ok: boolean; changed: number; message: string; exited: boolean }
export const roleName = (role: string) => role === 'owner' ? '群主' : role === 'admin' ? '管理员' : '成员'
export const memberError = (error: unknown) => (error as { message?: string })?.message?.replace(/^[A-Z_]+: /, '') || '无法连接服务器，请重试'

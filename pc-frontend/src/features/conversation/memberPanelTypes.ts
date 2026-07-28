import type { ProjectMember } from './types'

export type MemberModerationAction = 'mute' | 'unmute' | 'ban' | 'unban'
export type MemberMenuRequest = { member: ProjectMember; x: number; y: number }

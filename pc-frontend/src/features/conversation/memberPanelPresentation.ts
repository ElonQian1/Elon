import { styles } from './memberPanelStyles'

const BUILDER_ROLE_KEYS = ['developer', 'dev', 'maintainer', 'editor', 'collaborator', 'builder']
const OBSERVER_ROLE_KEYS = ['tester', 'qa', 'observer', 'viewer', 'guest']

export function memberRolePillClass(roleKey: string) {
  if (roleKey === 'owner') return styles.memberRolePillOwner
  if (roleKey === 'admin') return styles.memberRolePillAdmin
  if (BUILDER_ROLE_KEYS.includes(roleKey)) return styles.memberRolePillEditor
  if (OBSERVER_ROLE_KEYS.includes(roleKey)) return styles.memberRolePillObserver
  return ''
}

export function memberAvatarRoleClass(roleKey: string) {
  if (roleKey === 'owner') return styles.memberAvatarOwner
  if (roleKey === 'admin') return styles.memberAvatarAdmin
  if (BUILDER_ROLE_KEYS.includes(roleKey)) return styles.memberAvatarEditor
  if (OBSERVER_ROLE_KEYS.includes(roleKey)) return styles.memberAvatarObserver
  return ''
}

export function memberPresenceAvatarClass(status: string) {
  if (status === 'idle') return styles.memberAvatarIdle
  if (status === 'dnd') return styles.memberAvatarDnd
  if (status === 'offline') return styles.memberAvatarOffline
  return styles.memberAvatarOnline
}

export function memberPresencePillClass(status: string) {
  if (status === 'idle') return styles.memberPresencePillIdle
  if (status === 'dnd') return styles.memberPresencePillDnd
  if (status === 'offline') return styles.memberPresencePillOffline
  return styles.memberPresencePillOnline
}

export function localAiDirectoryAutoSyncKey({
  sessionIdentity,
  windowLabel,
  sessionOpen,
}: {
  sessionIdentity: string
  windowLabel?: string
  sessionOpen: boolean
}) {
  const identity = sessionIdentity.trim()
  if (!sessionOpen || !identity) return ''
  return `${identity}:${windowLabel?.trim() || 'session'}`
}

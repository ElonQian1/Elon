export type OneTimeProductionCredentialSecrets = Readonly<Record<string, string>>

export type OneTimeProductionCredentialAction =
  | { type: 'issue_started'; appRecordId: string }
  | { type: 'issue_succeeded'; appRecordId: string; liveToken: string }
  | { type: 'cleared'; appRecordId: string }

export function updateOneTimeProductionCredentialSecrets(
  current: OneTimeProductionCredentialSecrets,
  action: OneTimeProductionCredentialAction,
): OneTimeProductionCredentialSecrets {
  if (action.type === 'issue_succeeded') {
    const liveToken = action.liveToken.trim()
    if (!liveToken) return removeSecret(current, action.appRecordId)
    return { ...current, [action.appRecordId]: liveToken }
  }
  return removeSecret(current, action.appRecordId)
}

export function normalizeProductionCredentialRevocationReason(value: string): string {
  const reason = value.trim()
  const characterCount = [...reason].length
  if (!reason) return '项目方主动撤销生产凭据'
  if (characterCount < 4 || characterCount > 500 || [...reason].some((character) => /\p{Cc}/u.test(character))) {
    throw new Error('撤销原因需为 4 至 500 个可见字符。')
  }
  return reason
}

function removeSecret(
  current: OneTimeProductionCredentialSecrets,
  appRecordId: string,
): OneTimeProductionCredentialSecrets {
  const next = { ...current }
  delete next[appRecordId]
  return next
}

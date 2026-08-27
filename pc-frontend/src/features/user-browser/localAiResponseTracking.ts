export function normalizeLocalAiResponsePrompt(value: string): string {
  return value.trim().replace(/\s+/g, ' ')
}

export function lastMatchingLocalAiUserIndex(
  messages: Array<{ role: string; content: Array<{ type: string; text?: string }> }>,
  expected: string,
): number {
  for (let index = messages.length - 1; index >= 0; index -= 1) {
    const message = messages[index]
    if (message.role === 'user'
      && normalizeLocalAiResponsePrompt(visibleMessageText(message)) === expected) return index
  }
  return -1
}

export function matchingLocalAiUserCount(
  messages: Array<{ role: string; content: Array<{ type: string; text?: string }> }>,
  expected: string,
): number {
  const normalized = normalizeLocalAiResponsePrompt(expected)
  if (!normalized) return 0
  return messages.filter((message) => (
    message.role === 'user'
    && normalizeLocalAiResponsePrompt(visibleMessageText(message)) === normalized
  )).length
}

/** Finds the exact matching user turn that appeared after the send baseline. */
export function matchingLocalAiUserIndex(
  messages: Array<{ role: string; content: Array<{ type: string; text?: string }> }>,
  expected: string,
  baselineMatchingUserCount: number,
): number {
  const normalized = normalizeLocalAiResponsePrompt(expected)
  if (!normalized) return -1
  const target = Number.isFinite(baselineMatchingUserCount)
    ? Math.max(0, Math.floor(baselineMatchingUserCount))
    : 0
  let matchingUsers = 0
  for (let index = 0; index < messages.length; index += 1) {
    const message = messages[index]
    if (message.role !== 'user'
      || normalizeLocalAiResponsePrompt(visibleMessageText(message)) !== normalized) continue
    if (matchingUsers === target) return index
    matchingUsers += 1
  }
  return -1
}

function visibleMessageText(message: { content: Array<{ type: string; text?: string }> }): string {
  return message.content
    .filter((part) => part.type === 'text' || part.type === 'markdown')
    .map((part) => part.text ?? '')
    .join('\n')
}

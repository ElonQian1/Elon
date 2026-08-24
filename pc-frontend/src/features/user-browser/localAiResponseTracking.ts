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

function visibleMessageText(message: { content: Array<{ type: string; text?: string }> }): string {
  return message.content
    .filter((part) => part.type === 'text' || part.type === 'markdown')
    .map((part) => part.text ?? '')
    .join('\n')
}

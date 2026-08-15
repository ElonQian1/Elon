const REQUEST_ID_PATTERN = /^mcp_[a-z0-9]{1,32}$/
let requestSequence = 0

export function createLocalAiRequestId(): string {
  requestSequence = (requestSequence + 1) % 0x100000
  const time = Date.now().toString(36).slice(-8)
  const sequence = requestSequence.toString(36).padStart(4, '0')
  const random = randomToken(8)
  return `mcp_${time}${sequence}${random}`.slice(0, 36)
}

export function isLocalAiRequestId(value: unknown): value is string {
  return typeof value === 'string' && REQUEST_ID_PATTERN.test(value)
}

function randomToken(length: number): string {
  const bytes = new Uint8Array(length)
  try {
    globalThis.crypto?.getRandomValues(bytes)
  } catch {
    for (let index = 0; index < bytes.length; index += 1) {
      bytes[index] = Math.floor(Math.random() * 256)
    }
  }
  return Array.from(bytes, (value) => (value % 36).toString(36)).join('')
}

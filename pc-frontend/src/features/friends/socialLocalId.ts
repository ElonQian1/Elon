let sequence = 0

// Page-local queue identity only; never use this as a server ID or security token.
// HTTP-hosted Win workbenches do not expose crypto.randomUUID.
export function socialLocalId(): string {
  return `${Date.now().toString(36)}-${(++sequence).toString(36)}`
}

const positions = new Map<string, number>()
export function readSocialPosition(owner: string, conversation: string) { return positions.get(`${owner}:${conversation}`) }
export function saveSocialPosition(owner: string, conversation: string, top: number) {
  const key = `${owner}:${conversation}`
  positions.delete(key); positions.set(key, top)
  if (positions.size > 30) positions.delete(positions.keys().next().value!)
}

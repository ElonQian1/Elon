export function formatFen(value: number) {
  const sign = value < 0 ? '-' : ''
  const absolute = Math.abs(value)
  return `${sign}¥${Math.trunc(absolute / 100)}.${String(absolute % 100).padStart(2, '0')}`
}

export function formatMicros(value: number) {
  const sign = value < 0 ? '-' : ''
  const absolute = Math.abs(value)
  return `${sign}¥${Math.trunc(absolute / 1_000_000)}.${String(absolute % 1_000_000).padStart(6, '0')}`
}

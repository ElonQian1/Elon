import type { ConsumerRankingReceipt } from './openCommerceClientTypes'

export async function verifyConsumerRankingReceipt(receipt: ConsumerRankingReceipt) {
  if (!crypto.subtle) throw new Error('当前浏览器无法执行排序凭证 SHA-256 校验')
  if (receipt.hash_algorithm !== 'sha256') throw new Error('排序凭证摘要算法不受支持')
  const payloadBytes = new TextEncoder().encode(receipt.canonical_payload_json)
  const digestBytes = new Uint8Array(await crypto.subtle.digest('SHA-256', payloadBytes))
  const digest = Array.from(digestBytes, (byte) => byte.toString(16).padStart(2, '0')).join('')
  if (digest !== receipt.payload_sha256) throw new Error('排序凭证摘要校验失败')
  return true
}

export async function downloadConsumerRankingReceipt(receipt: ConsumerRankingReceipt) {
  await verifyConsumerRankingReceipt(receipt)
  const payload = JSON.parse(receipt.canonical_payload_json) as Record<string, unknown>
  const blob = new Blob([
    JSON.stringify({
      schema: receipt.schema,
      hash_algorithm: receipt.hash_algorithm,
      payload_sha256: receipt.payload_sha256,
      signed_by_operator: receipt.signed_by_operator,
      canonical_payload_json: receipt.canonical_payload_json,
      payload,
    }, null, 2),
  ], { type: 'application/json' })
  const url = URL.createObjectURL(blob)
  const anchor = document.createElement('a')
  anchor.href = url
  anchor.download = `open-commerce-ranking-${receipt.payload_sha256.slice(0, 16)}.json`
  document.body.appendChild(anchor)
  anchor.click()
  anchor.remove()
  window.setTimeout(() => URL.revokeObjectURL(url), 0)
}

import { FileCheck2 } from 'lucide-react'
import type { ConsumerDataErasureEvidence } from './openCommerceClientTypes'
import { commerceStyles } from './openCommerceStyles'

export default function DataErasureEvidenceList({
  evidence,
}: {
  evidence: ConsumerDataErasureEvidence[]
}) {
  if (evidence.length === 0) {
    return <p style={commerceStyles.itemText}>尚未附加外部删除证明。</p>
  }
  return (
    <div style={{ display: 'grid', gap: 6, paddingTop: 6, borderTop: '1px solid var(--line)' }}>
      {evidence.map((item) => (
        <div key={item.id} style={{ display: 'grid', gap: 3 }}>
          <strong style={{ display: 'inline-flex', alignItems: 'center', gap: 5, fontSize: 10 }}>
            <FileCheck2 size={12} />{item.external_system} · {evidenceKindLabel(item.evidence_kind)}
          </strong>
          <small style={commerceStyles.itemMeta}>回执 {item.reference_id} · SHA-256 {item.receipt_sha256.slice(0, 12)}…</small>
          <p style={commerceStyles.itemText}>{item.summary}</p>
          <small style={commerceStyles.itemMeta}>商户提交，平台未核验 · {new Date(item.created_at).toLocaleString('zh-CN')}</small>
        </div>
      ))}
    </div>
  )
}

function evidenceKindLabel(kind: ConsumerDataErasureEvidence['evidence_kind']) {
  return kind === 'external_system_receipt' ? '外部系统回执' : '商户声明'
}

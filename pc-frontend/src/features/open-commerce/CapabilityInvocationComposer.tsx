import { useEffect, useMemo, useState } from 'react'
import { Play, ShieldCheck, X } from 'lucide-react'
import CapabilitySchemaField from './CapabilitySchemaField'
import {
  asCapabilitySchema,
  buildCapabilityInput,
  capabilitySchemaSupportIssue,
  createCapabilityFormValue,
  type CapabilityInputIssue,
} from './capabilityInvocationSchema'
import type { ConsumerDiscoveryMatch } from './openCommerceClientTypes'
import { formatMicros } from './openCommerceUi'
import base from './OpenCommercePanel.module.css'
import {
  actionStyle,
  badgeStyle,
  commerceStyles,
  errorMessageStyle,
} from './openCommerceStyles'

interface CapabilityInvocationComposerProps {
  match: ConsumerDiscoveryMatch
  busy: boolean
  onCancel: () => void
  onInvoke: (
    match: ConsumerDiscoveryMatch,
    input: Record<string, unknown>,
    idempotencyKey: string,
  ) => Promise<boolean>
}

export default function CapabilityInvocationComposer({
  match,
  busy,
  onCancel,
  onInvoke,
}: CapabilityInvocationComposerProps) {
  const [formValue, setFormValue] = useState<unknown>(() => (
    createCapabilityFormValue(match.capability.input_schema)
  ))
  const [confirmed, setConfirmed] = useState(false)
  const [issues, setIssues] = useState<CapabilityInputIssue[]>([])
  const [idempotencyKey, setIdempotencyKey] = useState(createInvocationKey)
  const supportIssue = useMemo(
    () => capabilitySchemaSupportIssue(match.capability.input_schema),
    [match.capability.input_schema],
  )

  useEffect(() => {
    setFormValue(createCapabilityFormValue(match.capability.input_schema))
    setConfirmed(false)
    setIssues([])
    setIdempotencyKey(createInvocationKey())
  }, [match.capability.capability_key, match.capability.input_schema, match.merchant.id])

  async function submit(event: React.FormEvent) {
    event.preventDefault()
    if (supportIssue) {
      setIssues([{ path: '$', message: supportIssue }])
      return
    }
    const built = buildCapabilityInput(match.capability.input_schema, formValue)
    if (built.issues.length > 0 || !built.input) {
      setIssues(built.issues)
      return
    }
    if (match.capability.kind === 'action' && !confirmed) {
      setIssues([{ path: '$', message: '执行经营操作前需要本人明确确认。' }])
      return
    }
    setIssues([])
    const invoked = await onInvoke(match, built.input, idempotencyKey)
    if (invoked) {
      setConfirmed(false)
      onCancel()
    }
  }

  function changeFormValue(value: unknown) {
    setFormValue(value)
    setConfirmed(false)
    setIssues([])
    setIdempotencyKey(createInvocationKey())
  }

  return (
    <section className={base.integrationSection}>
      <header>
        <span>
          <strong>{match.merchant.display_name} · {match.capability.display_name}</strong>
          <small>{match.capability.description || match.capability.capability_key}</small>
        </span>
        <div style={commerceStyles.headerActions}>
          <span style={badgeStyle(match.capability.kind === 'action' ? 'warn' : 'neutral')}>
            {match.capability.kind === 'action' ? '经营操作' : '信息查询'}
          </span>
          <button type="button" style={actionStyle('icon', busy)} disabled={busy} title="关闭" onClick={onCancel}>
            <X size={14} />
          </button>
        </div>
      </header>

      <form className={base.formCard} style={{ ...commerceStyles.sectionBody, padding: 14 }} onSubmit={submit}>
        <div style={summaryStyle}>
          <span>技术服务计量</span>
          <strong>{formatMicros(match.capability.unit_price_micros, match.capability.currency)}</strong>
          <small>当前仅记录计量，未扣真实资金；商户商品或服务金额以商户返回结果为准。</small>
        </div>

        {supportIssue ? (
          <div style={{ ...commerceStyles.message, ...errorMessageStyle }}>{supportIssue}</div>
        ) : (
          <CapabilitySchemaField
            name="业务信息"
            path="$"
            schema={asCapabilitySchema(match.capability.input_schema)}
            value={formValue}
            required
            root
            onChange={changeFormValue}
          />
        )}

        {match.capability.kind === 'action' && (
          <label style={confirmationStyle}>
            <input
              type="checkbox"
              checked={confirmed}
              onChange={(event) => setConfirmed(event.target.checked)}
            />
            <ShieldCheck size={15} />
            <span>我确认执行“{match.capability.display_name}”，并理解该操作可能改变商户系统中的业务状态。</span>
          </label>
        )}

        {issues.length > 0 && (
          <div style={{ ...commerceStyles.message, ...errorMessageStyle }}>
            {issues.map((issue) => <div key={`${issue.path}:${issue.message}`}>{issue.path}：{issue.message}</div>)}
          </div>
        )}

        <footer style={footerStyle}>
          <button type="button" style={actionStyle('secondary', busy)} disabled={busy} onClick={onCancel}>取消</button>
          <button
            type="submit"
            style={actionStyle('primary', busy || Boolean(supportIssue))}
            disabled={busy || Boolean(supportIssue)}
          >
            <Play size={13} />
            {busy ? '执行中…' : match.capability.kind === 'action' ? '确认并执行' : '执行查询'}
          </button>
        </footer>
      </form>
    </section>
  )
}

function createInvocationKey(): string {
  return `consumer-sandbox-${crypto.randomUUID()}`
}

const summaryStyle: React.CSSProperties = {
  display: 'grid',
  gridTemplateColumns: 'minmax(0, 1fr) auto',
  gap: 4,
  paddingBottom: 10,
  borderBottom: '1px solid var(--line)',
  color: 'var(--text-muted)',
  fontSize: 10,
}
const confirmationStyle: React.CSSProperties = {
  display: 'flex',
  alignItems: 'flex-start',
  gap: 8,
  padding: 10,
  border: '1px solid #755c35',
  borderRadius: 6,
  color: '#f0c982',
  fontSize: 10,
  lineHeight: 1.5,
}
const footerStyle: React.CSSProperties = {
  display: 'flex',
  justifyContent: 'flex-end',
  gap: 8,
}

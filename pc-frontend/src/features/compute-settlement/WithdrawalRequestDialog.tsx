import { useMemo, useState, type FormEvent } from 'react'
import { CircleDollarSign, LoaderCircle, X } from 'lucide-react'
import {
  type CreateMyWithdrawalBody,
  type WithdrawalDestinationKind,
} from './myComputeSettlementApi'
import styles from './WithdrawalRequestDialog.module.css'

interface WithdrawalRequestDialogProps {
  availableMicros: number
  busy: boolean
  error: string
  onClose: () => void
  onSubmit: (body: CreateMyWithdrawalBody) => Promise<void>
}

const DESTINATIONS: Array<{ value: WithdrawalDestinationKind; label: string }> = [
  { value: 'bank_account_vault_ref', label: '银行目标金库引用' },
  { value: 'digital_wallet_vault_ref', label: '数字钱包金库引用' },
  { value: 'sui_address_ref', label: 'Sui 公开地址' },
  { value: 'other_vault_ref', label: '其他受控目标引用' },
]

export default function WithdrawalRequestDialog({
  availableMicros,
  busy,
  error,
  onClose,
  onSubmit,
}: WithdrawalRequestDialogProps) {
  const [amount, setAmount] = useState('')
  const [destinationKind, setDestinationKind] = useState<WithdrawalDestinationKind>('bank_account_vault_ref')
  const [destinationRef, setDestinationRef] = useState('')
  const [confirmedReserve, setConfirmedReserve] = useState(false)
  const [confirmedNoSecret, setConfirmedNoSecret] = useState(false)
  const [idempotencyKey] = useState(createIdempotencyKey)
  const amountMicros = useMemo(() => parseCnyToMicros(amount), [amount])
  const canSubmit = Boolean(
    amountMicros
    && amountMicros <= availableMicros
    && destinationRef.trim()
    && confirmedReserve
    && confirmedNoSecret
    && !busy,
  )

  async function submit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault()
    if (!canSubmit || !amountMicros) return
    await onSubmit({
      amount_micros: amountMicros,
      destination_kind: destinationKind,
      destination_ref: destinationRef.trim(),
      idempotency_key: idempotencyKey,
      confirm_internal_reserve_only: confirmedReserve,
      confirm_destination_ref_contains_no_secret: confirmedNoSecret,
    })
  }

  return (
    <div className={styles.backdrop} onMouseDown={(event) => event.target === event.currentTarget && !busy && onClose()}>
      <section className={styles.dialog} role="dialog" aria-modal="true" aria-labelledby="withdrawal-request-title">
        <header className={styles.header}>
          <div>
            <span>内部提款申请</span>
            <h2 id="withdrawal-request-title">申请提取算力收益</h2>
          </div>
          <button type="button" className={styles.iconButton} onClick={onClose} disabled={busy} aria-label="关闭" title="关闭">
            <X size={18} aria-hidden="true" />
          </button>
        </header>

        <div className={styles.available}>
          <CircleDollarSign size={19} aria-hidden="true" />
          <div><span>当前可申请</span><strong>{formatCny(availableMicros)}</strong></div>
        </div>

        <form onSubmit={(event) => void submit(event)}>
          {error && <div className={styles.error} role="alert">{error}</div>}
          <label className={styles.field}>
            <span>申请金额（元）</span>
            <input
              type="text"
              inputMode="decimal"
              value={amount}
              onChange={(event) => setAmount(event.target.value)}
              placeholder="0.00"
              autoFocus
              required
            />
            {amountMicros !== null && amountMicros > availableMicros && <small>申请金额超过当前 available 余额</small>}
          </label>
          <label className={styles.field}>
            <span>目标类型</span>
            <select value={destinationKind} onChange={(event) => setDestinationKind(event.target.value as WithdrawalDestinationKind)}>
              {DESTINATIONS.map((destination) => <option value={destination.value} key={destination.value}>{destination.label}</option>)}
            </select>
          </label>
          <label className={styles.field}>
            <span>目标引用</span>
            <input
              value={destinationRef}
              onChange={(event) => setDestinationRef(event.target.value)}
              maxLength={1000}
              placeholder={destinationKind === 'sui_address_ref' ? '公开的 Sui 地址' : '受控金库中的目标引用 ID'}
              required
            />
          </label>

          <div className={styles.confirmations}>
            <label><input type="checkbox" checked={confirmedReserve} onChange={(event) => setConfirmedReserve(event.target.checked)} /><span>我确认本操作只把 available 转入 withdrawn，不会立即对外付款</span></label>
            <label><input type="checkbox" checked={confirmedNoSecret} onChange={(event) => setConfirmedNoSecret(event.target.checked)} /><span>我确认目标引用不含密码、私钥、助记词或支付凭据</span></label>
          </div>

          <footer className={styles.footer}>
            <button type="button" className={styles.cancelButton} onClick={onClose} disabled={busy}>取消</button>
            <button type="submit" className={styles.submitButton} disabled={!canSubmit}>
              {busy && <LoaderCircle size={15} className={styles.spinning} aria-hidden="true" />}
              {busy ? '正在申请' : '确认申请'}
            </button>
          </footer>
        </form>
      </section>
    </div>
  )
}

function parseCnyToMicros(value: string) {
  const normalized = value.trim()
  const match = /^(\d+)(?:\.(\d{1,6}))?$/.exec(normalized)
  if (!match) return null
  const micros = Number(match[1]) * 1_000_000 + Number((match[2] ?? '').padEnd(6, '0'))
  return Number.isSafeInteger(micros) && micros > 0 ? micros : null
}

function createIdempotencyKey() {
  return `pc-provider-withdrawal:${globalThis.crypto.randomUUID()}`
}

function formatCny(micros: number) {
  return new Intl.NumberFormat('zh-CN', { style: 'currency', currency: 'CNY' }).format(micros / 1_000_000)
}

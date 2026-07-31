import { useCallback, useEffect, useState } from 'react'
import {
  FileJson,
  RefreshCw,
  Scale,
} from 'lucide-react'
import { taskEconomyApi } from './taskEconomyApi'
import type {
  SettlementReceipt,
  SettlementReceiptDetail,
  SuiSettlementEnvelope,
  TaskEconomyOverview,
} from './taskEconomyTypes'
import { errorText, formatMicros } from './openCommerceUi'
import base from './OpenCommercePanel.module.css'
import {
  actionStyle,
  badgeStyle,
  commerceStyles,
  errorMessageStyle,
  listItemStyle,
} from './openCommerceStyles'

export default function ShadowEconomyPanel({
  projectId,
  canEdit,
}: {
  projectId: string
  canEdit: boolean
}) {
  const [overview, setOverview] = useState<TaskEconomyOverview | null>(null)
  const [detail, setDetail] = useState<SettlementReceiptDetail | null>(null)
  const [envelope, setEnvelope] = useState<SuiSettlementEnvelope | null>(null)
  const [busy, setBusy] = useState(false)
  const [message, setMessage] = useState('')

  const refresh = useCallback(async () => {
    setMessage('')
    try {
      setOverview(await taskEconomyApi.overview(projectId))
    } catch (error) {
      setMessage(errorText(error))
    }
  }, [projectId])

  useEffect(() => {
    refresh()
  }, [refresh])

  async function toggleEnabled() {
    if (!overview) return
    setBusy(true)
    setMessage('')
    try {
      await taskEconomyApi.updateSetting(projectId, !overview.project_setting.enabled)
      await refresh()
    } catch (error) {
      setMessage(errorText(error))
    } finally {
      setBusy(false)
    }
  }

  async function selectReceipt(receipt: SettlementReceipt) {
    setBusy(true)
    setMessage('')
    setEnvelope(null)
    try {
      setDetail(await taskEconomyApi.receipt(projectId, receipt.id))
    } catch (error) {
      setMessage(errorText(error))
    } finally {
      setBusy(false)
    }
  }

  async function loadEnvelope() {
    if (!detail) return
    setBusy(true)
    setMessage('')
    try {
      setEnvelope(await taskEconomyApi.suiEnvelope(projectId, detail.receipt.id))
    } catch (error) {
      setMessage(errorText(error))
    } finally {
      setBusy(false)
    }
  }

  const totals = overview?.totals
  const ledgerState = detail ? reconcile(detail) : null

  return (
    <div className={base.panel}>
      <header className={base.hero} style={commerceStyles.workspaceHeader}>
        <div>
          <h2>影子经济与对账</h2>
          <p>复算 AI 任务和商业调用的经济结果；只生成链下凭证与 Sui 投影信封，不转移资金、不提交网络。</p>
        </div>
        <div style={commerceStyles.headerActions}>
          <span style={badgeStyle('warn')}>SHADOW ONLY</span>
          <button style={actionStyle('icon')} type="button" onClick={refresh} title="刷新">
            <RefreshCw size={15} />
          </button>
        </div>
      </header>

      <section className={base.stats}>
        <Metric label="用量凭证" value={totals?.usage_receipts ?? 0} detail="来自已记录执行" />
        <Metric label="待处理意图" value={totals?.pending_intents ?? 0} detail={`已过账 ${totals?.posted_intents ?? 0}`} />
        <Metric label="计算金额" value={formatMicros(totals?.compute_amount_micros ?? 0)} detail="影子计量" />
        <Metric label="节点 / 平台" value={`${formatShort(totals?.provider_amount_micros ?? 0)} / ${formatShort(totals?.platform_amount_micros ?? 0)}`} detail="CNY 影子分配" />
      </section>

      <section className={base.integrationSection}>
        <header>
          <strong>运行开关</strong>
          <div style={commerceStyles.headerActions}>
            <span style={badgeStyle(overview?.runtime_enabled ? 'neutral' : 'warn')}>服务端 {overview?.runtime_enabled ? '已启用' : '未启用'}</span>
            <span style={badgeStyle(overview?.project_setting.enabled ? 'neutral' : 'warn')}>项目 {overview?.project_setting.enabled ? '已启用' : '未启用'}</span>
          </div>
        </header>
        <div className={base.formCard} style={commerceStyles.sectionBody}>
          <label style={commerceStyles.checkRow}>
            <input type="checkbox" checked={overview?.project_setting.enabled ?? false} onChange={toggleEnabled} disabled={!canEdit || busy || !overview} />
            为当前项目生成新的影子经济凭证
          </label>
          {!overview?.runtime_enabled && <div style={commerceStyles.message}>项目设置已保存也不会绕过服务端运行开关。</div>}
        </div>
      </section>

      <div style={commerceStyles.grid}>
        <section className={base.integrationSection}>
          <header><strong>结算凭证</strong><span style={badgeStyle()}>{overview?.settlement_receipts.length ?? 0}</span></header>
          <div className={base.formCard} style={{ ...commerceStyles.sectionBody, ...commerceStyles.scrollArea }}>
            {overview?.settlement_receipts.map((receipt) => (
              <button
                className={base.formCard}
                data-selected={detail?.receipt.id === receipt.id}
                key={receipt.id}
                style={listItemStyle(detail?.receipt.id === receipt.id)}
                type="button"
                onClick={() => selectReceipt(receipt)}
              >
                <header style={commerceStyles.itemHeader}><h3 style={commerceStyles.itemTitle}>{receipt.reason}</h3><span style={badgeStyle(receipt.status === 'voided' ? 'warn' : 'neutral')}>{receipt.status}</span></header>
                <p style={commerceStyles.itemText}>{formatMicros(receipt.compute_amount_micros, receipt.currency)} · 节点 {formatShort(receipt.provider_amount_micros)} · 平台 {formatShort(receipt.platform_amount_micros)}</p>
                <code style={commerceStyles.itemMeta}>{receipt.posting_key}</code>
              </button>
            ))}
            {overview?.settlement_receipts.length === 0 && <p className={base.empty}>暂无影子结算凭证。</p>}
          </div>
        </section>

        <section className={base.integrationSection}>
          <header>
            <strong>凭证详情</strong>
            {ledgerState && <span style={badgeStyle(ledgerState.balanced ? 'neutral' : 'danger')}><Scale size={11} />{ledgerState.balanced ? '借贷平衡' : '借贷不平'}</span>}
          </header>
          <div className={base.formCard} style={commerceStyles.sectionBody}>
            {!detail && <p className={base.empty}>选择一张凭证查看来源与账本。</p>}
            {detail && (
              <>
                <article className={base.formCard} style={listItemStyle()}>
                  <header style={commerceStyles.itemHeader}><h3 style={commerceStyles.itemTitle}>{detail.receipt.id}</h3><span style={badgeStyle()}>{detail.receipt.status}</span></header>
                  <p style={commerceStyles.itemText}>策略 {detail.intent.policy_version} · 用量来源 {detail.usage_receipts.length} 条</p>
                  <code style={commerceStyles.itemMeta}>{detail.intent.policy_digest}</code>
                </article>
                <div style={commerceStyles.list}>
                  {detail.ledger_transaction?.entries.map((entry) => (
                    <div style={commerceStyles.priorityRow} key={entry.id}>
                      <span style={commerceStyles.priorityIndex}>{entry.side === 'debit' ? '借' : '贷'}</span>
                      <code>{entry.account_key}</code>
                      <strong>{formatShort(entry.amount_micros)}</strong>
                    </div>
                  ))}
                </div>
                <button style={actionStyle('secondary', busy || detail.receipt.status !== 'reconciled')} type="button" onClick={loadEnvelope} disabled={busy || detail.receipt.status !== 'reconciled'}>
                  <FileJson size={13} />生成 Sui 投影信封
                </button>
              </>
            )}
          </div>
        </section>
      </div>

      {envelope && (
        <section className={base.integrationSection}>
          <header><strong>Sui 投影信封</strong><span style={badgeStyle('warn')}>{envelope.network_submission}</span></header>
          <div className={base.formCard} style={commerceStyles.sectionBody}><pre className={base.result}>{JSON.stringify(envelope, null, 2)}</pre></div>
        </section>
      )}
      {message && <div style={{ ...commerceStyles.message, ...errorMessageStyle }}>{message}</div>}
    </div>
  )
}

function Metric({ label, value, detail }: { label: string; value: string | number; detail: string }) {
  return <div><span>{label}</span><strong>{value}</strong><small>{detail}</small></div>
}

function formatShort(value: number) {
  return (value / 1_000_000).toFixed(4)
}

function reconcile(detail: SettlementReceiptDetail) {
  const entries = detail.ledger_transaction?.entries ?? []
  const debit = entries.filter((entry) => entry.side === 'debit').reduce((sum, entry) => sum + entry.amount_micros, 0)
  const credit = entries.filter((entry) => entry.side === 'credit').reduce((sum, entry) => sum + entry.amount_micros, 0)
  return { debit, credit, balanced: entries.length > 0 && debit === credit }
}

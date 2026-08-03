import { useCallback, useEffect, useMemo, useState } from 'react'
import { Download, FileJson, PackagePlus, RefreshCw, ShieldCheck } from 'lucide-react'
import { taskEconomyApi } from './taskEconomyApi'
import { downloadSuiAdapterHandoff } from './suiAdapterHandoffDownload'
import type {
  SettlementReceipt,
  SuiProjectionPackage,
  SuiTargetNetwork,
} from './taskEconomyTypes'
import { errorText } from './openCommerceUi'
import base from './OpenCommercePanel.module.css'
import {
  actionStyle,
  badgeStyle,
  commerceStyles,
  errorMessageStyle,
  listItemStyle,
} from './openCommerceStyles'

export default function SuiProjectionPackages({
  projectId,
  canEdit,
  selectedReceipt,
  refreshToken,
}: {
  projectId: string
  canEdit: boolean
  selectedReceipt: SettlementReceipt | null
  refreshToken: number
}) {
  const [packages, setPackages] = useState<SuiProjectionPackage[]>([])
  const [selectedId, setSelectedId] = useState<string | null>(null)
  const [targetNetwork, setTargetNetwork] = useState<SuiTargetNetwork>('testnet')
  const [busy, setBusy] = useState(false)
  const [message, setMessage] = useState('')

  const refresh = useCallback(async () => {
    setMessage('')
    try {
      const next = await taskEconomyApi.suiProjections(projectId)
      setPackages(next)
      setSelectedId((current) =>
        current && next.some((item) => item.id === current) ? current : (next[0]?.id ?? null),
      )
    } catch (error) {
      setMessage(errorText(error))
    }
  }, [projectId])

  useEffect(() => {
    refresh()
  }, [refresh, refreshToken])

  const selected = useMemo(
    () => packages.find((item) => item.id === selectedId) ?? null,
    [packages, selectedId],
  )

  async function prepare() {
    if (!selectedReceipt) return
    setBusy(true)
    setMessage('')
    try {
      const prepared = await taskEconomyApi.prepareSuiProjection(
        projectId,
        selectedReceipt.id,
        targetNetwork,
      )
      await refresh()
      setSelectedId(prepared.id)
    } catch (error) {
      setMessage(errorText(error))
    } finally {
      setBusy(false)
    }
  }

  async function verify() {
    if (!selected) return
    setBusy(true)
    setMessage('')
    try {
      const verified = await taskEconomyApi.verifySuiProjection(projectId, selected.id)
      setPackages((current) => current.map((item) => (item.id === verified.id ? verified : item)))
    } catch (error) {
      setMessage(errorText(error))
    } finally {
      setBusy(false)
    }
  }

  async function downloadHandoff() {
    if (!selected) return
    setBusy(true)
    setMessage('')
    try {
      const bundle = await taskEconomyApi.suiProjectionAdapterHandoff(projectId, selected.id)
      downloadSuiAdapterHandoff(bundle)
      await refresh()
    } catch (error) {
      setMessage(errorText(error))
    } finally {
      setBusy(false)
    }
  }

  const isStandardReceipt = selectedReceipt?.receipt_kind === 'standard'
  const canPrepare =
    canEdit && selectedReceipt?.status === 'reconciled' && isStandardReceipt && !busy
  const prepareTitle = !selectedReceipt
    ? '请先选择已对账凭证'
    : !isStandardReceipt
      ? '纠正凭证必须作为冲销与替换组成的纠正包原子投影'
      : '保存所选凭证的不可变投影包'

  return (
    <section className={base.integrationSection}>
      <header>
        <strong>Sui 链下投影包</strong>
        <div style={commerceStyles.headerActions}>
          <span style={badgeStyle('warn')}>NOT SUBMITTED</span>
          <button style={actionStyle('icon')} type="button" onClick={refresh} title="刷新投影包">
            <RefreshCw size={14} />
          </button>
        </div>
      </header>
      <div style={commerceStyles.sectionBody}>
        <div style={commerceStyles.headerActions}>
          <select
            aria-label="Sui 目标网络"
            value={targetNetwork}
            onChange={(event) => setTargetNetwork(event.target.value as SuiTargetNetwork)}
            disabled={busy}
          >
            <option value="devnet">Devnet</option>
            <option value="testnet">Testnet</option>
            <option value="mainnet">Mainnet</option>
          </select>
          <button
            style={actionStyle('secondary', !canPrepare)}
            type="button"
            onClick={prepare}
            disabled={!canPrepare}
            title={prepareTitle}
          >
            <PackagePlus size={14} />保存投影包
          </button>
        </div>
        <div style={commerceStyles.grid}>
          <div style={{ ...commerceStyles.list, ...commerceStyles.scrollArea }}>
            {packages.map((item) => (
              <button
                className={base.formCard}
                key={item.id}
                style={listItemStyle(item.id === selectedId)}
                type="button"
                onClick={() => setSelectedId(item.id)}
              >
                <header style={commerceStyles.itemHeader}>
                  <strong>{item.target_network}</strong>
                  <span style={badgeStyle(item.integrity_status === 'verified' ? 'neutral' : 'danger')}>
                    {item.integrity_status}
                  </span>
                </header>
                <code style={commerceStyles.itemMeta}>{item.projection_digest.slice(0, 24)}</code>
              </button>
            ))}
            {packages.length === 0 && <p className={base.empty}>暂无投影包。</p>}
          </div>
          <div>
            {!selected && <p className={base.empty}>选择投影包查看摘要。</p>}
            {selected && (
              <>
                <div style={commerceStyles.priorityRow}>
                  <FileJson size={15} />
                  <span>{selected.submission_readiness}</span>
                  <strong>{selected.network_submission}</strong>
                </div>
                <code style={commerceStyles.itemMeta}>{selected.source_receipt_digest}</code>
                <button
                  style={actionStyle('secondary', !canEdit || busy)}
                  type="button"
                  onClick={verify}
                  disabled={!canEdit || busy}
                >
                  <ShieldCheck size={14} />复核完整性
                </button>
                <button
                  style={actionStyle(
                    'secondary',
                    !canEdit || busy || selected.submission_readiness !== 'adapter_required',
                  )}
                  type="button"
                  onClick={downloadHandoff}
                  disabled={!canEdit || busy || selected.submission_readiness !== 'adapter_required'}
                  title="重新复核并下载离线适配器交接包"
                >
                  <Download size={14} />下载交接包
                </button>
                <pre className={base.result}>{JSON.stringify(selected.envelope, null, 2)}</pre>
              </>
            )}
          </div>
        </div>
        {message && <div style={{ ...commerceStyles.message, ...errorMessageStyle }}>{message}</div>}
      </div>
    </section>
  )
}

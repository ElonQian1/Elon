import { useCallback, useEffect, useState } from 'react'
import { Copy, KeyRound, RefreshCw, RotateCw, ShieldOff } from 'lucide-react'
import { taskEconomyApi } from './taskEconomyApi'
import type {
  SuiPreflightAdapter,
  SuiPreflightPackageKind,
  SuiPreflightReport,
} from './suiPreflightTypes'
import { errorText } from './openCommerceUi'
import base from './OpenCommercePanel.module.css'
import {
  actionStyle,
  badgeStyle,
  commerceStyles,
  errorMessageStyle,
  listItemStyle,
} from './openCommerceStyles'

type Network = 'devnet' | 'testnet' | 'mainnet'

export default function SuiPreflightAdaptersPanel({
  projectId,
  canEdit,
}: {
  projectId: string
  canEdit: boolean
}) {
  const [adapters, setAdapters] = useState<SuiPreflightAdapter[]>([])
  const [reports, setReports] = useState<SuiPreflightReport[]>([])
  const [runtimeEnabled, setRuntimeEnabled] = useState(false)
  const [displayName, setDisplayName] = useState('离线预检适配器')
  const [expiresInDays, setExpiresInDays] = useState(30)
  const [networks, setNetworks] = useState<Network[]>(['testnet'])
  const [packageKinds, setPackageKinds] = useState<SuiPreflightPackageKind[]>([
    'standard',
    'correction',
  ])
  const [issuedToken, setIssuedToken] = useState('')
  const [busy, setBusy] = useState(false)
  const [message, setMessage] = useState('')

  const refresh = useCallback(async () => {
    setMessage('')
    try {
      const [adapterList, reportList] = await Promise.all([
        taskEconomyApi.suiPreflightAdapters(projectId),
        taskEconomyApi.suiPreflightReports(projectId),
      ])
      setAdapters(adapterList.adapters)
      setReports(reportList.reports)
      setRuntimeEnabled(adapterList.runtime_enabled && reportList.runtime_enabled)
    } catch (error) {
      setMessage(errorText(error))
    }
  }, [projectId])

  useEffect(() => {
    refresh()
  }, [refresh])

  async function createAdapter() {
    if (!window.confirm('确认签发新的 Sui 离线预检机器凭据？明文只显示一次。')) return
    setBusy(true)
    setMessage('')
    setIssuedToken('')
    try {
      const issue = await taskEconomyApi.createSuiPreflightAdapter(projectId, {
        display_name: displayName,
        allowed_networks: networks,
        allowed_package_kinds: packageKinds,
        expires_in_days: expiresInDays,
        confirmed_by_user: true,
      })
      setIssuedToken(issue.adapter_token)
      await refresh()
    } catch (error) {
      setMessage(errorText(error))
    } finally {
      setBusy(false)
    }
  }

  async function rotateAdapter(adapter: SuiPreflightAdapter) {
    if (!window.confirm(`确认轮换“${adapter.display_name}”的凭据？旧凭据将立即失效。`)) return
    setBusy(true)
    setMessage('')
    setIssuedToken('')
    try {
      const issue = await taskEconomyApi.rotateSuiPreflightAdapter(
        projectId,
        adapter.id,
        expiresInDays,
      )
      setIssuedToken(issue.adapter_token)
      await refresh()
    } catch (error) {
      setMessage(errorText(error))
    } finally {
      setBusy(false)
    }
  }

  async function disableAdapter(adapter: SuiPreflightAdapter) {
    if (!window.confirm(`确认停用“${adapter.display_name}”？该操作不会删除历史报告。`)) return
    setBusy(true)
    setMessage('')
    try {
      await taskEconomyApi.disableSuiPreflightAdapter(projectId, adapter.id)
      await refresh()
    } catch (error) {
      setMessage(errorText(error))
    } finally {
      setBusy(false)
    }
  }

  async function copyIssuedToken() {
    await navigator.clipboard.writeText(issuedToken)
    setMessage('凭据已复制；关闭本页后平台无法再次显示明文。')
  }

  return (
    <section className={base.integrationSection}>
      <header>
        <strong>Sui 离线预检适配器</strong>
        <div style={commerceStyles.headerActions}>
          <span style={badgeStyle(runtimeEnabled ? 'neutral' : 'warn')}>
            {runtimeEnabled ? 'REPORTING ON' : 'REPORTING OFF'}
          </span>
          <button style={actionStyle('icon')} type="button" onClick={refresh} title="刷新">
            <RefreshCw size={14} />
          </button>
        </div>
      </header>
      <div style={commerceStyles.sectionBody}>
        <div className={base.formCard} style={commerceStyles.sectionBody}>
          <input
            aria-label="适配器名称"
            value={displayName}
            onChange={(event) => setDisplayName(event.target.value)}
            disabled={!canEdit || busy}
          />
          <label style={commerceStyles.checkRow}>
            有效天数
            <input
              aria-label="凭据有效天数"
              type="number"
              min={1}
              max={366}
              value={expiresInDays}
              onChange={(event) => setExpiresInDays(Number(event.target.value))}
              disabled={!canEdit || busy}
            />
          </label>
          <div style={commerceStyles.headerActions}>
            {(['devnet', 'testnet', 'mainnet'] as Network[]).map((network) => (
              <label style={commerceStyles.checkRow} key={network}>
                <input
                  type="checkbox"
                  checked={networks.includes(network)}
                  onChange={() => setNetworks(toggleValue(networks, network))}
                  disabled={!canEdit || busy}
                />
                {network}
              </label>
            ))}
          </div>
          <div style={commerceStyles.headerActions}>
            {(['standard', 'correction'] as SuiPreflightPackageKind[]).map((kind) => (
              <label style={commerceStyles.checkRow} key={kind}>
                <input
                  type="checkbox"
                  checked={packageKinds.includes(kind)}
                  onChange={() => setPackageKinds(toggleValue(packageKinds, kind))}
                  disabled={!canEdit || busy}
                />
                {kind}
              </label>
            ))}
          </div>
          <button
            style={actionStyle('primary', !canEdit || busy)}
            type="button"
            onClick={createAdapter}
            disabled={!canEdit || busy}
          >
            <KeyRound size={14} />签发凭据
          </button>
        </div>

        {issuedToken && (
          <div className={base.formCard} style={listItemStyle(true)}>
            <header style={commerceStyles.itemHeader}>
              <strong>仅本次显示</strong>
              <button style={actionStyle('icon')} type="button" onClick={copyIssuedToken} title="复制凭据">
                <Copy size={14} />
              </button>
            </header>
            <code style={commerceStyles.itemMeta}>{issuedToken}</code>
          </div>
        )}

        <div style={commerceStyles.grid}>
          <div style={{ ...commerceStyles.list, ...commerceStyles.scrollArea }}>
            {adapters.map((adapter) => (
              <article className={base.formCard} style={listItemStyle()} key={adapter.id}>
                <header style={commerceStyles.itemHeader}>
                  <strong>{adapter.display_name}</strong>
                  <span style={badgeStyle(adapter.status === 'active' && !adapter.is_expired ? 'neutral' : 'warn')}>
                    {adapter.is_expired ? 'expired' : adapter.status}
                  </span>
                </header>
                <p style={commerceStyles.itemText}>
                  {adapter.allowed_networks.join(' / ')} · {adapter.allowed_package_kinds.join(' / ')}
                </p>
                <code style={commerceStyles.itemMeta}>v{adapter.credential_version} {adapter.token_hint}</code>
                {adapter.status === 'active' && (
                  <div style={commerceStyles.headerActions}>
                    <button
                      style={actionStyle('icon')}
                      type="button"
                      onClick={() => rotateAdapter(adapter)}
                      disabled={!canEdit || busy}
                      title="轮换凭据"
                    >
                      <RotateCw size={14} />
                    </button>
                    <button
                      style={actionStyle('icon')}
                      type="button"
                      onClick={() => disableAdapter(adapter)}
                      disabled={!canEdit || busy}
                      title="停用适配器"
                    >
                      <ShieldOff size={14} />
                    </button>
                  </div>
                )}
              </article>
            ))}
            {adapters.length === 0 && <p className={base.empty}>暂无离线预检适配器。</p>}
          </div>

          <div style={{ ...commerceStyles.list, ...commerceStyles.scrollArea }}>
            {reports.map((report) => (
              <article className={base.formCard} style={listItemStyle()} key={report.id}>
                <header style={commerceStyles.itemHeader}>
                  <strong>{report.package_kind} · {report.target_network}</strong>
                  <span style={badgeStyle(report.outcome === 'passed' ? 'neutral' : 'danger')}>
                    {report.outcome}
                  </span>
                </header>
                <p style={commerceStyles.itemText}>{report.summary}</p>
                <code style={commerceStyles.itemMeta}>{report.projection_package_id}</code>
                <code style={commerceStyles.itemMeta}>{report.report_digest.slice(0, 24)}</code>
              </article>
            ))}
            {reports.length === 0 && <p className={base.empty}>暂无机器预检报告。</p>}
          </div>
        </div>
        {message && <div style={{ ...commerceStyles.message, ...errorMessageStyle }}>{message}</div>}
      </div>
    </section>
  )
}

function toggleValue<T>(values: T[], value: T): T[] {
  return values.includes(value) ? values.filter((item) => item !== value) : [...values, value]
}

import { useMemo, useState } from 'react'
import { openCommerceApi } from './openCommerceApi'
import type {
  OpenCommerceCapability,
  OpenCommerceCapabilitySourceLink,
  OpenCommerceIntegration,
  OpenCommerceSyncReceipt,
} from './openCommerceTypes'
import { commerceStyles } from './openCommerceStyles'
import OpenCommerceAdapterCredentialManager from './OpenCommerceAdapterCredentialManager'
import styles from './OpenCommercePanel.module.css'

interface Props {
  projectId: string
  merchantId: string
  integrations: OpenCommerceIntegration[]
  receipts: OpenCommerceSyncReceipt[]
  capabilities: OpenCommerceCapability[]
  sourceLinks: OpenCommerceCapabilitySourceLink[]
  canEdit: boolean
  onChanged: () => Promise<void>
}

export default function OpenCommerceIntegrationManager({
  projectId,
  merchantId,
  integrations,
  receipts,
  capabilities,
  sourceLinks,
  canEdit,
  onChanged,
}: Props) {
  const [displayName, setDisplayName] = useState('')
  const [providerKey, setProviderKey] = useState('')
  const [integrationKey, setIntegrationKey] = useState('')
  const [connectionMode, setConnectionMode] =
    useState<OpenCommerceIntegration['connection_mode']>('merchant_export')
  const [scopes, setScopes] = useState('read.orders')
  const [dataDomains, setDataDomains] = useState('orders')
  const [sourceCapabilityId, setSourceCapabilityId] = useState('')
  const [sourceReceiptId, setSourceReceiptId] = useState('')
  const [sourceDataDomain, setSourceDataDomain] = useState('')
  const [busy, setBusy] = useState('')
  const [message, setMessage] = useState('')

  const merchantIntegrations = useMemo(
    () => integrations.filter((integration) => integration.merchant_id === merchantId),
    [integrations, merchantId],
  )
  const merchantIntegrationIds = useMemo(
    () => new Set(merchantIntegrations.map((integration) => integration.id)),
    [merchantIntegrations],
  )
  const eligibleReceipts = useMemo(
    () => receipts.filter((receipt) =>
      merchantIntegrationIds.has(receipt.integration_id)
      && receipt.sync_kind !== 'health_check'
      && ['succeeded', 'partial'].includes(receipt.status)),
    [merchantIntegrationIds, receipts],
  )
  const selectedSourceReceipt = eligibleReceipts.find(
    (receipt) => receipt.id === sourceReceiptId,
  )
  const selectedSourceIntegration = merchantIntegrations.find(
    (integration) => integration.id === selectedSourceReceipt?.integration_id,
  )

  async function createIntegration(event: React.FormEvent) {
    event.preventDefault()
    setBusy('create')
    setMessage('')
    try {
      await openCommerceApi.createIntegration(projectId, {
        merchant_id: merchantId,
        integration_key: integrationKey,
        provider_key: providerKey,
        display_name: displayName,
        connection_mode: connectionMode,
        scopes: splitList(scopes),
        data_domains: splitList(dataDomains),
      })
      setDisplayName('')
      setProviderKey('')
      setIntegrationKey('')
      setMessage('数据接入已登记。只有适配器提交成功回执后才会显示为已连接。')
      await onChanged()
    } catch (error) {
      setMessage(errorMessage(error))
    } finally {
      setBusy('')
    }
  }

  async function toggleIntegration(integration: OpenCommerceIntegration) {
    setBusy(integration.id)
    setMessage('')
    try {
      await openCommerceApi.setIntegrationEnabled(
        projectId,
        integration.id,
        integration.status === 'disabled',
      )
      await onChanged()
    } catch (error) {
      setMessage(errorMessage(error))
    } finally {
      setBusy('')
    }
  }

  async function linkCapabilitySource(event: React.FormEvent) {
    event.preventDefault()
    if (!sourceCapabilityId || !selectedSourceReceipt || !sourceDataDomain) {
      setMessage('请选择能力、同步回执和数据域。')
      return
    }
    setBusy('link-source')
    setMessage('')
    try {
      await openCommerceApi.linkCapabilitySource(projectId, sourceCapabilityId, {
        integration_id: selectedSourceReceipt.integration_id,
        sync_receipt_id: selectedSourceReceipt.id,
        data_domain: sourceDataDomain,
      })
      setMessage('能力已关联内部同步回执；该声明不代表外部平台验证。')
      setSourceCapabilityId('')
      setSourceReceiptId('')
      setSourceDataDomain('')
      await onChanged()
    } catch (error) {
      setMessage(errorMessage(error))
    } finally {
      setBusy('')
    }
  }

  async function removeCapabilitySource(link: OpenCommerceCapabilitySourceLink) {
    setBusy(`unlink-source:${link.id}`)
    setMessage('')
    try {
      await openCommerceApi.removeCapabilitySource(projectId, link.capability_id)
      setMessage('能力来源回执绑定已移除。')
      await onChanged()
    } catch (error) {
      setMessage(errorMessage(error))
    } finally {
      setBusy('')
    }
  }

  return (
    <section className={styles.integrationSection}>
      <header>
        <span>
          <strong>经营数据接入</strong>
          <small>只保存授权范围、数据域和同步证据，不保存平台令牌</small>
        </span>
        <em>{merchantIntegrations.length} 个来源</em>
      </header>

      <div className={styles.integrationGrid}>
        <div className={styles.integrationList}>
          {merchantIntegrations.map((integration) => {
            const latestReceipt = receipts.find(
              (receipt) => receipt.integration_id === integration.id,
            )
            return (
              <article key={integration.id}>
                <div>
                  <strong>{integration.display_name}</strong>
                  <code>{integration.provider_key} · {modeLabel(integration.connection_mode)}</code>
                </div>
                <span data-status={integration.status}>{statusLabel(integration.status)}</span>
                <p>数据域：{integration.data_domains.join('、') || '未声明'}</p>
                <small>
                  {latestReceipt
                    ? `最近回执：${receiptLabel(latestReceipt.status)} · ${new Date(latestReceipt.completed_at).toLocaleString('zh-CN')}`
                    : '尚无同步或健康检查回执'}
                </small>
                <footer>
                  <small>连接器运行后自动提交回执</small>
                  <button
                    type="button"
                    onClick={() => toggleIntegration(integration)}
                    disabled={!canEdit || busy !== ''}
                  >
                    {integration.status === 'disabled' ? '启用' : '停用'}
                  </button>
                </footer>
              </article>
            )
          })}
          {merchantIntegrations.length === 0 && (
            <p className={styles.empty}>尚未登记美团、抖音、收银系统或其他数据来源。</p>
          )}
        </div>

        <form className={styles.formCard} onSubmit={createIntegration}>
          <header>
            <strong>登记数据来源</strong>
            <small>登记不等于平台已经开放官方 API</small>
          </header>
          <label>显示名称<input value={displayName} onChange={(event) => setDisplayName(event.target.value)} required disabled={!canEdit} /></label>
          <div className={styles.twoColumns}>
            <label>平台标识<input value={providerKey} onChange={(event) => setProviderKey(event.target.value)} placeholder="meituan" required disabled={!canEdit} /></label>
            <label>接入键<input value={integrationKey} onChange={(event) => setIntegrationKey(event.target.value)} placeholder="meituan.main" required disabled={!canEdit} /></label>
          </div>
          <label>接入方式<select value={connectionMode} onChange={(event) => setConnectionMode(event.target.value as OpenCommerceIntegration['connection_mode'])} disabled={!canEdit}>
            <option value="official_api">官方 API</option>
            <option value="merchant_export">商户授权导出</option>
            <option value="local_adapter">本地适配器</option>
            <option value="manual_import">手工导入</option>
          </select></label>
          <label>授权范围（逗号分隔）<input value={scopes} onChange={(event) => setScopes(event.target.value)} disabled={!canEdit} /></label>
          <label>数据域（逗号分隔）<input value={dataDomains} onChange={(event) => setDataDomains(event.target.value)} placeholder="orders,inventory" disabled={!canEdit} /></label>
          <button type="submit" disabled={!canEdit || busy !== ''}>
            {busy === 'create' ? '登记中…' : '登记数据来源'}
          </button>
        </form>
      </div>
      <div className={styles.integrationGrid}>
        <div className={styles.integrationList}>
          {sourceLinks.map((link) => (
            <article key={link.id}>
              <div>
                <strong>{capabilityLabel(capabilities, link.capability_id)}</strong>
                <code>{link.provider_key} · {link.data_domain}</code>
              </div>
              <span data-status={link.publishable ? 'connected' : 'degraded'}>
                {link.publishable ? '可公开' : '需重绑'}
              </span>
              <p>回执：{receiptLabel(link.receipt_status)} · {link.sync_kind}</p>
              <small>
                {new Date(link.receipt_completed_at).toLocaleString('zh-CN')} · SHA-256 {link.receipt_sha256.slice(0, 16)}…
              </small>
              <footer>
                <small>{link.publishable ? '内部回执，未经外部平台验证' : blockingReasonLabel(link.blocking_reason)}</small>
                <button
                  type="button"
                  onClick={() => removeCapabilitySource(link)}
                  disabled={!canEdit || busy !== ''}
                >
                  移除
                </button>
              </footer>
            </article>
          ))}
          {sourceLinks.length === 0 && (
            <p className={styles.empty}>尚未将公开能力关联到内部同步回执。</p>
          )}
        </div>

        <form className={styles.formCard} onSubmit={linkCapabilitySource}>
          <header>
            <strong>声明能力数据来源</strong>
            <small>绑定项目内部回执，不代表外部平台签名或回读验证</small>
          </header>
          <label>商业能力<select value={sourceCapabilityId} onChange={(event) => setSourceCapabilityId(event.target.value)} required disabled={!canEdit}>
            <option value="">选择能力</option>
            {capabilities.map((capability) => (
              <option key={capability.id} value={capability.id}>{capability.display_name} · v{capability.version}</option>
            ))}
          </select></label>
          <label>同步回执<select value={sourceReceiptId} onChange={(event) => {
            setSourceReceiptId(event.target.value)
            setSourceDataDomain('')
          }} required disabled={!canEdit}>
            <option value="">选择成功回执</option>
            {eligibleReceipts.map((receipt) => {
              const integration = merchantIntegrations.find((item) => item.id === receipt.integration_id)
              return <option key={receipt.id} value={receipt.id}>{integration?.display_name ?? receipt.integration_id} · {receiptLabel(receipt.status)} · {new Date(receipt.completed_at).toLocaleString('zh-CN')}</option>
            })}
          </select></label>
          <label>数据域<select value={sourceDataDomain} onChange={(event) => setSourceDataDomain(event.target.value)} required disabled={!canEdit || !selectedSourceIntegration}>
            <option value="">选择该接入的数据域</option>
            {(selectedSourceIntegration?.data_domains ?? []).map((domain) => (
              <option key={domain} value={domain}>{domain}</option>
            ))}
          </select></label>
          <button type="submit" disabled={!canEdit || busy !== '' || !sourceCapabilityId || !sourceReceiptId || !sourceDataDomain}>
            {busy === 'link-source' ? '绑定中…' : '绑定来源回执'}
          </button>
        </form>
      </div>
      <OpenCommerceAdapterCredentialManager
        projectId={projectId}
        integrations={merchantIntegrations}
        canEdit={canEdit}
      />
      {message && <div style={commerceStyles.message}>{message}</div>}
    </section>
  )
}

function splitList(value: string) {
  return value.split(',').map((item) => item.trim()).filter(Boolean)
}

function statusLabel(status: OpenCommerceIntegration['status']) {
  const labels = {
    configured: '待验证',
    connected: '已连接',
    degraded: '异常',
    disabled: '已停用',
  }
  return labels[status]
}

function modeLabel(mode: OpenCommerceIntegration['connection_mode']) {
  const labels = {
    official_api: '官方 API',
    merchant_export: '授权导出',
    local_adapter: '本地适配器',
    manual_import: '手工导入',
  }
  return labels[mode]
}

function receiptLabel(status: OpenCommerceSyncReceipt['status']) {
  if (status === 'succeeded') return '成功'
  if (status === 'partial') return '部分成功'
  return '失败'
}

function capabilityLabel(capabilities: OpenCommerceCapability[], capabilityId: string) {
  const capability = capabilities.find((item) => item.id === capabilityId)
  return capability ? `${capability.display_name} · v${capability.version}` : capabilityId
}

function blockingReasonLabel(reason?: string) {
  if (reason === 'capability_version_changed') return '能力版本已变化，请重新绑定'
  if (reason === 'integration_disabled') return '数据接入已停用'
  if (reason === 'health_check_not_business_data') return '健康检查不能作为业务来源'
  if (reason === 'receipt_not_eligible') return '回执状态不再符合公开条件'
  return '当前绑定不可公开'
}

function errorMessage(error: unknown) {
  if (error instanceof Error) return error.message
  if (error && typeof error === 'object' && 'message' in error) return String(error.message)
  return '操作失败，请稍后重试'
}

import { useMemo, useState } from 'react'
import { openCommerceApi } from './openCommerceApi'
import type {
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
  canEdit: boolean
  onChanged: () => Promise<void>
}

export default function OpenCommerceIntegrationManager({
  projectId,
  merchantId,
  integrations,
  receipts,
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
  const [busy, setBusy] = useState('')
  const [message, setMessage] = useState('')

  const merchantIntegrations = useMemo(
    () => integrations.filter((integration) => integration.merchant_id === merchantId),
    [integrations, merchantId],
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

function errorMessage(error: unknown) {
  if (error instanceof Error) return error.message
  if (error && typeof error === 'object' && 'message' in error) return String(error.message)
  return '操作失败，请稍后重试'
}

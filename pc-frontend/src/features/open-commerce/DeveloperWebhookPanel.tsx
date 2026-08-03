import { useCallback, useEffect, useMemo, useState } from 'react'
import {
  Copy,
  KeyRound,
  Power,
  PowerOff,
  RefreshCw,
  RotateCcw,
  ShieldCheck,
  Webhook,
} from 'lucide-react'
import { openCommerceClientApi } from './openCommerceClientApi'
import DeveloperWebhookReplayControls from './DeveloperWebhookReplayControls'
import type {
  DeveloperWebhookDelivery,
  DeveloperWebhookHistoryReplayResult,
  DeveloperWebhookSubscription,
  OpenCommerceDeveloperApp,
} from './openCommerceClientTypes'
import { errorText } from './openCommerceUi'
import base from './OpenCommercePanel.module.css'
import {
  actionStyle,
  badgeStyle,
  commerceStyles,
  listItemStyle,
} from './openCommerceStyles'

function webhookStatusLabel(webhook: DeveloperWebhookSubscription) {
  if (webhook.verification_status === 'pending') return '待验证'
  if (webhook.verification_status === 'failed') return '验证失败'
  return webhook.status === 'active' ? '投递中' : '已停用'
}

export default function DeveloperWebhookPanel({
  projectId,
  apps,
  canEdit,
}: {
  projectId: string
  apps: OpenCommerceDeveloperApp[]
  canEdit: boolean
}) {
  const [appRecordId, setAppRecordId] = useState('')
  const [callbackUrl, setCallbackUrl] = useState('')
  const [deliverOnSucceeded, setDeliverOnSucceeded] = useState(true)
  const [deliverOnFailed, setDeliverOnFailed] = useState(true)
  const [webhooks, setWebhooks] = useState<DeveloperWebhookSubscription[]>([])
  const [deliveries, setDeliveries] = useState<DeveloperWebhookDelivery[]>([])
  const [selectedWebhookId, setSelectedWebhookId] = useState('')
  const [visibleSecret, setVisibleSecret] = useState('')
  const [busy, setBusy] = useState(false)
  const [message, setMessage] = useState('')

  const selectedApp = useMemo(
    () => apps.find((app) => app.id === appRecordId),
    [appRecordId, apps],
  )
  const selectedWebhook = useMemo(
    () => webhooks.find((webhook) => webhook.id === selectedWebhookId),
    [selectedWebhookId, webhooks],
  )

  useEffect(() => {
    if (!apps.some((app) => app.id === appRecordId)) {
      setAppRecordId(apps.find((app) => app.status === 'active')?.id ?? apps[0]?.id ?? '')
    }
  }, [appRecordId, apps])

  const refreshWebhooks = useCallback(async () => {
    if (!appRecordId) {
      setWebhooks([])
      return
    }
    try {
      const response = await openCommerceClientApi.listDeveloperWebhooks(projectId, appRecordId)
      setWebhooks(response.webhooks)
      if (!response.webhooks.some((item) => item.id === selectedWebhookId)) {
        setSelectedWebhookId(response.webhooks[0]?.id ?? '')
      }
    } catch (error) {
      setMessage(errorText(error))
    }
  }, [appRecordId, projectId, selectedWebhookId])

  useEffect(() => {
    refreshWebhooks()
  }, [refreshWebhooks])

  const refreshDeliveries = useCallback(async () => {
    if (!appRecordId || !selectedWebhookId) {
      setDeliveries([])
      return
    }
    try {
      const response = await openCommerceClientApi.listDeveloperWebhookDeliveries(
        projectId,
        appRecordId,
        selectedWebhookId,
      )
      setDeliveries(response.deliveries)
    } catch (error) {
      setMessage(errorText(error))
    }
  }, [appRecordId, projectId, selectedWebhookId])

  useEffect(() => {
    refreshDeliveries()
  }, [refreshDeliveries])

  async function createWebhook(event: React.FormEvent) {
    event.preventDefault()
    if (!selectedApp) return
    setBusy(true)
    setMessage('')
    setVisibleSecret('')
    try {
      const credential = await openCommerceClientApi.createDeveloperWebhook(
        projectId,
        selectedApp.id,
        callbackUrl,
        deliverOnSucceeded,
        deliverOnFailed,
      )
      setVisibleSecret(credential.signing_secret)
      setSelectedWebhookId(credential.subscription.id)
      setCallbackUrl('')
      setMessage('签名密钥只显示本次。保存密钥并配置接收端后，请验证回调地址。')
      await refreshWebhooks()
    } catch (error) {
      setMessage(errorText(error))
    } finally {
      setBusy(false)
    }
  }

  async function setEnabled(webhook: DeveloperWebhookSubscription, enabled: boolean) {
    if (!selectedApp) return
    setBusy(true)
    setMessage('')
    try {
      if (enabled) {
        await openCommerceClientApi.enableDeveloperWebhook(projectId, selectedApp.id, webhook.id)
      } else {
        await openCommerceClientApi.disableDeveloperWebhook(projectId, selectedApp.id, webhook.id)
      }
      await refreshWebhooks()
      await refreshDeliveries()
    } catch (error) {
      setMessage(errorText(error))
    } finally {
      setBusy(false)
    }
  }

  async function verifyWebhook(webhook: DeveloperWebhookSubscription) {
    if (!selectedApp) return
    setBusy(true)
    setMessage('')
    try {
      await openCommerceClientApi.verifyDeveloperWebhook(projectId, selectedApp.id, webhook.id)
      setMessage('回调地址验证通过，后续终态事件将开始签名投递。')
      await refreshWebhooks()
      await refreshDeliveries()
    } catch (error) {
      setMessage(errorText(error))
      await refreshWebhooks()
    } finally {
      setBusy(false)
    }
  }

  async function rotateSecret(webhook: DeveloperWebhookSubscription) {
    if (!selectedApp) return
    setBusy(true)
    setMessage('')
    setVisibleSecret('')
    try {
      const credential = await openCommerceClientApi.rotateDeveloperWebhookSecret(
        projectId,
        selectedApp.id,
        webhook.id,
      )
      setVisibleSecret(credential.signing_secret)
      setSelectedWebhookId(webhook.id)
      setMessage('旧密钥已失效。新密钥只显示本次，接收端更新后需要重新验证。')
      await refreshWebhooks()
      await refreshDeliveries()
    } catch (error) {
      setMessage(errorText(error))
    } finally {
      setBusy(false)
    }
  }

  async function retryDelivery(delivery: DeveloperWebhookDelivery) {
    if (!selectedApp || !selectedWebhookId) return
    setBusy(true)
    setMessage('')
    try {
      await openCommerceClientApi.retryDeveloperWebhookDelivery(
        projectId,
        selectedApp.id,
        selectedWebhookId,
        delivery.id,
      )
      setMessage('死信已重新进入投递队列。')
      await refreshDeliveries()
    } catch (error) {
      setMessage(errorText(error))
    } finally {
      setBusy(false)
    }
  }

  async function replayHistory(
    afterSequence: number,
    limit: number,
  ): Promise<DeveloperWebhookHistoryReplayResult | undefined> {
    if (!selectedApp || !selectedWebhookId) return undefined
    setBusy(true)
    setMessage('')
    try {
      const replay = await openCommerceClientApi.replayDeveloperWebhookHistory(
        projectId,
        selectedApp.id,
        selectedWebhookId,
        afterSequence,
        limit,
      )
      setMessage(
        `符合 ${replay.eligible_count} 条，新入队 ${replay.enqueued_count} 条，`
        + `已存在 ${replay.already_present_count} 条${replay.has_more ? '，还有下一批' : ''}。`,
      )
      await refreshDeliveries()
      return replay
    } catch (error) {
      setMessage(errorText(error))
      return undefined
    } finally {
      setBusy(false)
    }
  }

  return (
    <section className={base.integrationSection}>
      <header>
        <strong>终态事件 Webhook</strong>
        <button
          style={actionStyle('icon')}
          type="button"
          onClick={() => { refreshWebhooks(); refreshDeliveries() }}
          title="刷新 Webhook"
        >
          <RefreshCw size={14} />
        </button>
      </header>
      <div className={base.formCard} style={commerceStyles.sectionBody}>
        <form style={commerceStyles.grid} onSubmit={createWebhook}>
          <label>
            开发者 App
            <select value={appRecordId} onChange={(event) => setAppRecordId(event.target.value)}>
              {apps.map((app) => (
                <option key={app.id} value={app.id}>{app.display_name} · {app.app_id}</option>
              ))}
            </select>
          </label>
          <label style={commerceStyles.wideField}>
            HTTPS 回调地址
            <input
              type="url"
              value={callbackUrl}
              onChange={(event) => setCallbackUrl(event.target.value)}
              placeholder="https://example.com/webhooks/open-commerce"
              disabled={!canEdit || selectedApp?.status !== 'active'}
              required
            />
          </label>
          <div style={commerceStyles.itemHeader}>
            <label>
              <input
                type="checkbox"
                checked={deliverOnSucceeded}
                onChange={(event) => setDeliverOnSucceeded(event.target.checked)}
              />
              调用成功
            </label>
            <label>
              <input
                type="checkbox"
                checked={deliverOnFailed}
                onChange={(event) => setDeliverOnFailed(event.target.checked)}
              />
              调用失败
            </label>
          </div>
          <button
            style={actionStyle(
              'primary',
              !canEdit
                || busy
                || selectedApp?.status !== 'active'
                || (!deliverOnSucceeded && !deliverOnFailed),
            )}
            type="submit"
            disabled={
              !canEdit
              || busy
              || selectedApp?.status !== 'active'
              || (!deliverOnSucceeded && !deliverOnFailed)
            }
          >
            <Webhook size={13} />创建订阅
          </button>
        </form>

        {visibleSecret && (
          <div style={commerceStyles.grid}>
            <label style={commerceStyles.wideField}>
              一次性签名密钥
              <div style={commerceStyles.itemHeader}>
                <input value={visibleSecret} readOnly />
                <button
                  style={actionStyle('icon')}
                  type="button"
                  onClick={() => navigator.clipboard.writeText(visibleSecret)}
                  title="复制签名密钥"
                >
                  <Copy size={14} />
                </button>
              </div>
            </label>
          </div>
        )}

        <div style={commerceStyles.list}>
          {webhooks.map((webhook) => (
            <article className={base.formCard} style={listItemStyle()} key={webhook.id}>
              <header style={commerceStyles.itemHeader}>
                <button
                  style={actionStyle('secondary')}
                  type="button"
                  onClick={() => setSelectedWebhookId(webhook.id)}
                >
                  {new URL(webhook.callback_url).host}
                </button>
                <span style={badgeStyle(webhook.status === 'active' ? 'neutral' : 'warn')}>
                  {webhookStatusLabel(webhook)}
                </span>
              </header>
              <code style={commerceStyles.itemMeta}>{webhook.callback_url}</code>
              <small style={commerceStyles.itemMeta}>
                失败 {webhook.consecutive_failures} 次 ·{' '}
                {webhook.verification_error_code ?? webhook.last_error_code ?? '无错误'}
              </small>
              <small style={commerceStyles.itemMeta}>
                接收事件：{[
                  webhook.deliver_on_succeeded ? '调用成功' : '',
                  webhook.deliver_on_failed ? '调用失败' : '',
                ].filter(Boolean).join('、')}
              </small>
              <footer style={commerceStyles.itemHeader}>
                <small style={commerceStyles.itemMeta}>
                  密钥 v{webhook.signing_secret_version} · {webhook.signing_key_id.slice(0, 18)}
                </small>
                <div style={commerceStyles.itemHeader}>
                  <button
                    style={actionStyle('icon', !canEdit || busy || selectedApp?.status !== 'active')}
                    type="button"
                    onClick={() => rotateSecret(webhook)}
                    disabled={!canEdit || busy || selectedApp?.status !== 'active'}
                    title="轮换签名密钥"
                  >
                    <KeyRound size={13} />
                  </button>
                  {webhook.verification_status !== 'verified' ? (
                    <button
                      style={actionStyle('primary', !canEdit || busy || selectedApp?.status !== 'active')}
                      type="button"
                      onClick={() => verifyWebhook(webhook)}
                      disabled={!canEdit || busy || selectedApp?.status !== 'active'}
                      title="验证回调地址"
                    >
                      <ShieldCheck size={13} />验证
                    </button>
                  ) : webhook.status === 'active' ? (
                    <button
                      style={actionStyle('icon', !canEdit || busy)}
                      type="button"
                      onClick={() => setEnabled(webhook, false)}
                      disabled={!canEdit || busy}
                      title="停用 Webhook"
                    >
                      <PowerOff size={13} />
                    </button>
                  ) : (
                    <button
                      style={actionStyle('icon', !canEdit || busy || selectedApp?.status !== 'active')}
                      type="button"
                      onClick={() => setEnabled(webhook, true)}
                      disabled={!canEdit || busy || selectedApp?.status !== 'active'}
                      title="启用 Webhook"
                    >
                      <Power size={13} />
                    </button>
                  )}
                </div>
              </footer>
            </article>
          ))}
          {webhooks.length === 0 && <p className={base.empty}>当前 App 还没有 Webhook。</p>}
        </div>

        {selectedWebhookId && (
          <div style={commerceStyles.list}>
            <header style={commerceStyles.itemHeader}>
              <strong>最近投递</strong>
              <DeveloperWebhookReplayControls
                disabled={
                  !canEdit
                  || busy
                  || selectedWebhook?.status !== 'active'
                  || selectedWebhook?.verification_status !== 'verified'
                }
                onReplay={replayHistory}
                onMessage={setMessage}
              />
            </header>
            {deliveries.slice(0, 10).map((delivery) => (
              <article style={listItemStyle()} key={delivery.id}>
                <header style={commerceStyles.itemHeader}>
                  <code style={commerceStyles.itemMeta}>{delivery.event_type}</code>
                  <div style={commerceStyles.itemHeader}>
                    <span style={badgeStyle(delivery.status === 'delivered' ? 'neutral' : 'warn')}>
                      {delivery.status}
                    </span>
                    {delivery.status === 'dead' && (
                      <button
                        style={actionStyle('icon', !canEdit || busy)}
                        type="button"
                        onClick={() => retryDelivery(delivery)}
                        disabled={!canEdit || busy}
                        title="重新投递死信"
                      >
                        <RotateCcw size={13} />
                      </button>
                    )}
                  </div>
                </header>
                <small style={commerceStyles.itemMeta}>
                  序号 {delivery.event_sequence} · {delivery.invocation_id} · 尝试 {delivery.attempt_count} 次
                  {delivery.enqueue_source === 'history_replay' ? ' · 历史补发' : ''}
                  {delivery.manual_retry_count > 0 ? ` · 人工重试 ${delivery.manual_retry_count} 次` : ''}
                  {delivery.response_status ? ` · HTTP ${delivery.response_status}` : ''}
                </small>
              </article>
            ))}
            {deliveries.length === 0 && <p className={base.empty}>暂无终态事件投递。</p>}
          </div>
        )}
        {message && <div style={commerceStyles.message}>{message}</div>}
      </div>
    </section>
  )
}

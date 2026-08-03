import { useCallback, useEffect, useState } from 'react'
import {
  Check,
  Copy,
  KeyRound,
  Play,
  Power,
  PowerOff,
  RefreshCw,
  RotateCcw,
  X,
} from 'lucide-react'
import { openCommerceClientApi } from './openCommerceClientApi'
import {
  optionalPositiveInteger,
  optionalYuanMicros,
} from './openCommerceGrantBudget'
import {
  grantExpiresAt,
  grantExpiryOptions,
  grantTermsLabel,
  type GrantExpiryPreset,
} from './openCommerceGrantExpiry'
import OutboundAuthorizationRequests from './OutboundAuthorizationRequests'
import DeveloperInvocationEvents from './DeveloperInvocationEvents'
import DeveloperWebhookPanel from './DeveloperWebhookPanel'
import type {
  AuthorizationRequest,
  OpenCommerceDeveloperApp,
} from './openCommerceClientTypes'
import { errorText, parseJsonObject } from './openCommerceUi'
import base from './OpenCommercePanel.module.css'
import {
  actionStyle,
  badgeStyle,
  commerceStyles,
  listItemStyle,
} from './openCommerceStyles'

export default function DeveloperCommercePortal({
  projectId,
  canEdit,
}: {
  projectId: string
  canEdit: boolean
}) {
  const [apps, setApps] = useState<OpenCommerceDeveloperApp[]>([])
  const [requests, setRequests] = useState<AuthorizationRequest[]>([])
  const [outboundRequests, setOutboundRequests] = useState<AuthorizationRequest[]>([])
  const [appId, setAppId] = useState('')
  const [displayName, setDisplayName] = useState('')
  const [visibleToken, setVisibleToken] = useState('')
  const [testToken, setTestToken] = useState('')
  const [merchantId, setMerchantId] = useState('')
  const [capabilityKey, setCapabilityKey] = useState('')
  const [grantId, setGrantId] = useState('')
  const [input, setInput] = useState('{}')
  const [confirmAction, setConfirmAction] = useState(false)
  const [approvalMaxInvocations, setApprovalMaxInvocations] = useState('')
  const [approvalMaxAmountYuan, setApprovalMaxAmountYuan] = useState('')
  const [approvalExpiryPreset, setApprovalExpiryPreset] = useState<GrantExpiryPreset>('30')
  const [response, setResponse] = useState<Record<string, unknown> | null>(null)
  const [eventRefreshKey, setEventRefreshKey] = useState(0)
  const [busy, setBusy] = useState(false)
  const [message, setMessage] = useState('')

  const refresh = useCallback(async () => {
    try {
      const [appResponse, requestResponse, outboundResponse] = await Promise.all([
        openCommerceClientApi.listApps(projectId),
        openCommerceClientApi.listAuthorizationRequests(projectId),
        openCommerceClientApi.listOutboundAuthorizationRequests(projectId),
      ])
      setApps(appResponse.apps)
      setRequests(requestResponse.requests)
      setOutboundRequests(outboundResponse.requests)
    } catch (error) {
      setMessage(errorText(error))
    }
  }, [projectId])

  useEffect(() => {
    refresh()
  }, [refresh])

  async function createApp(event: React.FormEvent) {
    event.preventDefault()
    setBusy(true)
    setMessage('')
    try {
      const credential = await openCommerceClientApi.createApp(projectId, {
        app_id: appId,
        display_name: displayName,
      })
      showCredential(credential.test_token)
      setAppId('')
      setDisplayName('')
      await refresh()
    } catch (error) {
      setMessage(errorText(error))
    } finally {
      setBusy(false)
    }
  }

  async function rotateToken(app: OpenCommerceDeveloperApp) {
    setBusy(true)
    setMessage('')
    try {
      const credential = await openCommerceClientApi.rotateToken(projectId, app.id)
      showCredential(credential.test_token)
      await refresh()
    } catch (error) {
      setMessage(errorText(error))
    } finally {
      setBusy(false)
    }
  }

  async function disableApp(app: OpenCommerceDeveloperApp) {
    setBusy(true)
    setMessage('')
    try {
      await openCommerceClientApi.disableApp(projectId, app.id)
      setTestToken('')
      setVisibleToken('')
      setMessage('应用已停用，旧测试凭据已永久失效，待处理授权申请已自动撤回。')
      await refresh()
    } catch (error) {
      setMessage(errorText(error))
    } finally {
      setBusy(false)
    }
  }

  async function reactivateApp(app: OpenCommerceDeveloperApp) {
    setBusy(true)
    setMessage('')
    try {
      const credential = await openCommerceClientApi.reactivateApp(projectId, app.id)
      showCredential(credential.test_token)
      await refresh()
    } catch (error) {
      setMessage(errorText(error))
    } finally {
      setBusy(false)
    }
  }

  async function cancelOutbound(request: AuthorizationRequest) {
    setBusy(true)
    setMessage('')
    try {
      await openCommerceClientApi.cancelOutboundAuthorization(projectId, request.id)
      setMessage('授权申请已撤回，商户收件箱会同步显示撤回状态。')
      await refresh()
    } catch (error) {
      setMessage(errorText(error))
    } finally {
      setBusy(false)
    }
  }

  async function decide(request: AuthorizationRequest, decision: 'approve' | 'reject') {
    setBusy(true)
    setMessage('')
    try {
      const approvalBudget = decision === 'approve' ? {
        expires_at: grantExpiresAt(approvalExpiryPreset),
        max_invocations: optionalPositiveInteger(approvalMaxInvocations, '总调用次数'),
        max_amount_micros: optionalYuanMicros(approvalMaxAmountYuan),
        budget_currency: 'CNY',
      } : {}
      await openCommerceClientApi.decideAuthorization(
        projectId,
        request.id,
        decision,
        {
          reason: decision === 'approve' ? '商户项目批准沙盒用途' : '商户项目拒绝沙盒用途',
          ...approvalBudget,
        },
      )
      await refresh()
    } catch (error) {
      setMessage(errorText(error))
    } finally {
      setBusy(false)
    }
  }

  async function invoke(event: React.FormEvent) {
    event.preventDefault()
    setBusy(true)
    setMessage('')
    setResponse(null)
    try {
      const request = {
        merchant_id: merchantId.trim(),
        capability_key: capabilityKey.trim(),
        grant_id: grantId.trim() || undefined,
        idempotency_key: `developer-console-${crypto.randomUUID()}`,
        input: parseJsonObject(input),
      }
      let actionConfirmationId: string | undefined
      if (confirmAction) {
        const prepared = await openCommerceClientApi.developerPrepareActionConfirmation(
          testToken.trim(),
          request,
        )
        const confirmed = await openCommerceClientApi.developerConfirmActionConfirmation(
          testToken.trim(),
          prepared.id,
        )
        actionConfirmationId = confirmed.id
      }
      const result = await openCommerceClientApi.developerInvoke(testToken.trim(), {
        ...request,
        action_confirmation_id: actionConfirmationId,
      })
      setResponse(result)
      setEventRefreshKey((current) => current + 1)
    } catch (error) {
      setMessage(errorText(error))
    } finally {
      setBusy(false)
    }
  }

  function showCredential(token: string) {
    setVisibleToken(token)
    setTestToken(token)
    setMessage('测试凭据只显示本次，请立即保存；轮换或重新启用会使此前凭据失效。')
  }

  return (
    <div className={base.panel}>
      <header className={base.hero} style={commerceStyles.workspaceHeader}>
        <div>
          <h2>第三方应用开发者门户</h2>
          <p>沙盒 App、一次性测试凭据、授权收件箱、能力调试和可恢复结果流。</p>
        </div>
        <button style={actionStyle('icon')} type="button" onClick={() => { setMessage(''); refresh() }} title="刷新">
          <RefreshCw size={15} />
        </button>
      </header>

      {visibleToken && (
        <section className={base.integrationSection}>
          <header>
            <strong>一次性测试凭据</strong>
            <button style={actionStyle('icon')} type="button" onClick={() => navigator.clipboard.writeText(visibleToken)} title="复制凭据">
              <Copy size={14} />
            </button>
          </header>
          <div className={base.formCard} style={commerceStyles.sectionBody}><pre className={base.result}>{visibleToken}</pre></div>
        </section>
      )}

      <div style={commerceStyles.grid}>
        <section className={base.integrationSection}>
          <header><strong>沙盒应用</strong><span style={badgeStyle()}>{apps.length}</span></header>
          <div className={base.formCard} style={commerceStyles.sectionBody}>
            <form style={commerceStyles.grid} onSubmit={createApp}>
              <label>App ID<input value={appId} onChange={(event) => setAppId(event.target.value)} placeholder="consumer.demo" disabled={!canEdit} required /></label>
              <label>应用名称<input value={displayName} onChange={(event) => setDisplayName(event.target.value)} disabled={!canEdit} required /></label>
              <button style={actionStyle('primary', !canEdit || busy)} type="submit" disabled={!canEdit || busy}><KeyRound size={13} />注册应用</button>
            </form>
            <div style={commerceStyles.list}>
              {apps.map((app) => (
                <article className={base.formCard} style={listItemStyle()} key={app.id}>
                  <header style={commerceStyles.itemHeader}><h3 style={commerceStyles.itemTitle}>{app.display_name}</h3><span style={badgeStyle(app.status === 'active' ? 'neutral' : 'warn')}>{app.status === 'active' ? '已启用' : '已停用'}</span></header>
                  <code style={commerceStyles.itemMeta}>{app.app_id} · {app.token_hint}</code>
                  <footer style={commerceStyles.itemHeader}>
                    <small style={commerceStyles.itemMeta}>{app.environment}</small>
                    <div style={commerceStyles.headerActions}>
                      {app.status === 'active' ? (
                        <>
                          <button style={actionStyle('icon', !canEdit || busy)} type="button" onClick={() => rotateToken(app)} disabled={!canEdit || busy} title="轮换测试凭据"><RotateCcw size={13} /></button>
                          <button style={actionStyle('icon', !canEdit || busy)} type="button" onClick={() => disableApp(app)} disabled={!canEdit || busy} title="停用应用"><PowerOff size={13} /></button>
                        </>
                      ) : (
                        <button style={actionStyle('icon', !canEdit || busy)} type="button" onClick={() => reactivateApp(app)} disabled={!canEdit || busy} title="重新启用并生成新凭据"><Power size={13} /></button>
                      )}
                    </div>
                  </footer>
                </article>
              ))}
              {apps.length === 0 && <p className={base.empty}>当前项目还没有沙盒应用。</p>}
            </div>
          </div>
        </section>

        <section className={base.integrationSection}>
          <header><strong>商户授权收件箱</strong><span style={badgeStyle('warn')}>{requests.filter((item) => item.status === 'pending').length} 待处理</span></header>
          <div className={base.formCard} style={{ ...commerceStyles.sectionBody, ...commerceStyles.scrollArea }}>
            <div style={commerceStyles.grid}>
              <label>批准后有效期<select value={approvalExpiryPreset} onChange={(event) => setApprovalExpiryPreset(event.target.value as GrantExpiryPreset)} disabled={!canEdit}>
                {grantExpiryOptions.map((option) => <option key={option.value} value={option.value}>{option.label}</option>)}
              </select></label>
              <label>批准后总调用次数<input type="number" min="1" value={approvalMaxInvocations} onChange={(event) => setApprovalMaxInvocations(event.target.value)} placeholder="留空不限" disabled={!canEdit} /></label>
              <label>批准后总预算（元）<input type="number" min="0.000001" step="0.000001" value={approvalMaxAmountYuan} onChange={(event) => setApprovalMaxAmountYuan(event.target.value)} placeholder="留空不限" disabled={!canEdit} /></label>
            </div>
            {requests.map((request) => (
              <article className={base.formCard} style={listItemStyle()} key={request.id}>
                <header style={commerceStyles.itemHeader}><h3 style={commerceStyles.itemTitle}>{request.requester_app_id}</h3><span style={badgeStyle(request.status === 'pending' ? 'warn' : 'neutral')}>{request.status}</span></header>
                <p style={commerceStyles.itemText}>{request.purpose}</p>
                <code style={commerceStyles.itemMeta}>{request.scopes.join(', ')}</code>
                {request.status === 'approved' && (
                  <small style={commerceStyles.itemMeta}>{grantTermsLabel({
                    expires_at: request.grant_expires_at,
                    max_invocations: request.grant_max_invocations,
                    max_amount_micros: request.grant_max_amount_micros,
                  })}</small>
                )}
                {request.status === 'pending' && (
                  <footer style={commerceStyles.itemHeader}>
                    <small style={commerceStyles.itemMeta}>{request.merchant_id}</small>
                    <div style={commerceStyles.headerActions}>
                      <button style={actionStyle('danger', !canEdit || busy)} type="button" onClick={() => decide(request, 'reject')} disabled={!canEdit || busy}><X size={13} />拒绝</button>
                      <button style={actionStyle('primary', !canEdit || busy)} type="button" onClick={() => decide(request, 'approve')} disabled={!canEdit || busy}><Check size={13} />批准</button>
                    </div>
                  </footer>
                )}
              </article>
            ))}
            {requests.length === 0 && <p className={base.empty}>暂无授权申请。</p>}
          </div>
        </section>
      </div>

      <OutboundAuthorizationRequests requests={outboundRequests} canEdit={canEdit} busy={busy} onCancel={cancelOutbound} />

      <section className={base.integrationSection}>
        <header><strong>能力调用调试器</strong><span style={badgeStyle()}>TEST TOKEN</span></header>
        <form className={base.formCard} style={commerceStyles.sectionBody} onSubmit={invoke}>
          <div style={commerceStyles.grid}>
            <label style={commerceStyles.wideField}>测试凭据<input type="password" value={testToken} onChange={(event) => setTestToken(event.target.value)} required /></label>
            <label>商户 ID<input value={merchantId} onChange={(event) => setMerchantId(event.target.value)} required /></label>
            <label>能力 Key<input value={capabilityKey} onChange={(event) => setCapabilityKey(event.target.value)} required /></label>
            <label>Grant ID<input value={grantId} onChange={(event) => setGrantId(event.target.value)} /></label>
            <label style={commerceStyles.wideField}>输入 JSON<textarea value={input} onChange={(event) => setInput(event.target.value)} /></label>
            <label style={commerceStyles.wideField}><input type="checkbox" checked={confirmAction} onChange={(event) => setConfirmAction(event.target.checked)} />本次是动作能力，我已确认当前输入并同意执行</label>
          </div>
          <button style={actionStyle('primary', busy)} type="submit" disabled={busy}><Play size={13} />{busy ? '调用中…' : '执行沙盒调用'}</button>
          {response && <pre className={base.result}>{JSON.stringify(response, null, 2)}</pre>}
        </form>
      </section>

      <DeveloperInvocationEvents testToken={testToken} refreshKey={eventRefreshKey} />

      <DeveloperWebhookPanel projectId={projectId} apps={apps} canEdit={canEdit} />

      {message && <div style={commerceStyles.message}>{message}</div>}
    </div>
  )
}

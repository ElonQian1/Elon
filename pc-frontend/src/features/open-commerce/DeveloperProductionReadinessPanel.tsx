import { useCallback, useEffect, useState } from 'react'
import { CheckCircle2, CircleAlert, RefreshCw } from 'lucide-react'
import { openCommerceClientApi } from './openCommerceClientApi'
import type { OpenCommerceDeveloperApp } from './openCommerceClientTypes'
import type {
  DeveloperProductionReadinessStepCode,
  DeveloperProductionReadinessSummary,
} from './developerProductionReadinessTypes'
import { errorText } from './openCommerceUi'
import base from './OpenCommercePanel.module.css'
import { actionStyle, badgeStyle, commerceStyles, listItemStyle } from './openCommerceStyles'

const stepLabels: Record<DeveloperProductionReadinessStepCode, string> = {
  app: 'App 状态',
  manifest: '资料审核',
  domain: '当前域名证明',
  admission: '网络准入',
  credential_gateway: '生产凭据开关',
  credential: '当前生产凭据',
  webhook_gateway: '生产 Webhook 开关',
  webhook: '生产 Webhook 订阅',
}

const blockerLabels: Record<string, string> = {
  app_inactive: '请先重新启用 App',
  manifest_not_approved: '请完成当前资料审核',
  domain_not_verified_for_current_revision: '请验证当前资料对应的主页域名',
  admission_not_approved_for_current_revision: '请提交并通过当前修订的网络准入',
  production_credentials_disabled: '运营方尚未开启生产凭据入口',
  current_production_credential_missing: '请签发当前修订的生产凭据',
  production_webhooks_disabled: '运营方尚未开启生产 Webhook',
  active_production_webhook_missing: '请创建、验证并启用生产 Webhook',
}

export default function DeveloperProductionReadinessPanel({
  projectId,
  apps,
  canEdit,
}: {
  projectId: string
  apps: OpenCommerceDeveloperApp[]
  canEdit: boolean
}) {
  const [appRecordId, setAppRecordId] = useState('')
  const [summary, setSummary] = useState<DeveloperProductionReadinessSummary | null>(null)
  const [message, setMessage] = useState('')
  const [loading, setLoading] = useState(false)

  useEffect(() => {
    if (!apps.some((app) => app.id === appRecordId)) {
      setAppRecordId(apps[0]?.id ?? '')
    }
  }, [appRecordId, apps])

  const refresh = useCallback(async () => {
    if (!appRecordId || !canEdit) {
      setSummary(null)
      return
    }
    setLoading(true)
    try {
      setSummary(await openCommerceClientApi.developerProductionReadiness(projectId, appRecordId))
      setMessage('')
    } catch (error) {
      setSummary(null)
      setMessage(errorText(error))
    } finally {
      setLoading(false)
    }
  }, [appRecordId, canEdit, projectId])

  useEffect(() => {
    refresh()
  }, [refresh])

  return (
    <section className={base.integrationSection}>
      <header>
        <strong>生产就绪</strong>
        <div style={commerceStyles.headerActions}>
          {summary && (
            <>
              <span style={badgeStyle(summary.production_invocation_ready ? 'neutral' : 'warn')}>
                调用{summary.production_invocation_ready ? '就绪' : '未就绪'}
              </span>
              <span style={badgeStyle(summary.production_webhook_ready ? 'neutral' : 'warn')}>
                通知{summary.production_webhook_ready ? '就绪' : '未就绪'}
              </span>
            </>
          )}
          <button
            style={actionStyle('icon', loading || !appRecordId)}
            type="button"
            onClick={refresh}
            disabled={loading || !appRecordId}
            title="刷新生产就绪状态"
          >
            <RefreshCw size={13} />
          </button>
        </div>
      </header>
      <div className={base.formCard} style={commerceStyles.sectionBody}>
        <label>
          开发者 App
          <select value={appRecordId} onChange={(event) => setAppRecordId(event.target.value)}>
            {apps.map((app) => (
              <option key={app.id} value={app.id}>{app.display_name} · {app.app_id}</option>
            ))}
          </select>
        </label>
        {summary && (
          <div style={commerceStyles.list}>
            {summary.steps.map((step) => (
              <div style={listItemStyle()} key={step.code}>
                <header style={commerceStyles.itemHeader}>
                  <span style={commerceStyles.itemTitle}>{stepLabels[step.code]}</span>
                  {step.ready ? <CheckCircle2 size={14} /> : <CircleAlert size={14} />}
                </header>
                {!step.ready && step.blocker_code && (
                  <small style={commerceStyles.itemMeta}>
                    {blockerLabels[step.blocker_code] ?? step.blocker_code}
                  </small>
                )}
              </div>
            ))}
            <small style={commerceStyles.itemMeta}>
              R{summary.manifest_revision} · 生产 Webhook {summary.active_production_webhook_count} 个
            </small>
          </div>
        )}
        {!summary && !message && <p className={base.empty}>请选择开发者 App。</p>}
        {message && <div style={commerceStyles.message}>{message}</div>}
      </div>
    </section>
  )
}

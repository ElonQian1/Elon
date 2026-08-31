import { useCallback, useEffect, useMemo, useState } from 'react'
import {
  CheckCircle2,
  Copy,
  ExternalLink,
  LoaderCircle,
  RefreshCw,
  Share2,
  ShieldCheck,
  Smartphone,
} from 'lucide-react'
import { useAuthStore } from '../../store/auth'
import { openCommerceApi } from './openCommerceApi'
import { openCommerceClientApi } from './openCommerceClientApi'
import type { OpenCommerceDeveloperApp } from './openCommerceClientTypes'
import type {
  OpenCommerceGrant,
  OpenCommerceMerchantDetail,
  OpenCommerceRuntimeBinding,
} from './openCommerceTypes'
import {
  MOBILE_CAPTURE_CAPABILITY,
  MOBILE_CAPTURE_TARGET,
  assertCompatibleMobileCaptureCapability,
  mobileCaptureCapabilityDefinition,
  parseMobileCaptureInvocation,
  selectUsableMobileCaptureGrant,
  type MobileCaptureLaunch,
} from './merchantMobileCaptureProtocol.js'
import styles from './MerchantMobileCapturePanel.module.css'

const APP_DISPLAY_NAME = '手机商户平台绑定'

interface Props {
  projectId: string
  merchant: OpenCommerceMerchantDetail
  grants: OpenCommerceGrant[]
  runtimeBinding?: OpenCommerceRuntimeBinding
  canEdit: boolean
  onChanged: () => Promise<void>
}

export default function MerchantMobileCapturePanel({
  projectId,
  merchant,
  grants,
  runtimeBinding,
  canEdit,
  onChanged,
}: Props) {
  const userId = useAuthStore((state) => state.user?.id)
  const [apps, setApps] = useState<OpenCommerceDeveloperApp[]>([])
  const [loadingApps, setLoadingApps] = useState(true)
  const [busy, setBusy] = useState(false)
  const [message, setMessage] = useState('')
  const [error, setError] = useState(false)
  const [launch, setLaunch] = useState<MobileCaptureLaunch | null>(null)

  const capability = merchant.capabilities.find(
    (item) => item.capability_key === MOBILE_CAPTURE_CAPABILITY,
  )
  const ownedApps = useMemo(
    () => apps
      .filter((app) => app.owner_user_id === userId && app.display_name === APP_DISPLAY_NAME)
      .sort((left, right) => Date.parse(right.created_at) - Date.parse(left.created_at)),
    [apps, userId],
  )
  const activeApp = ownedApps.find((app) => app.status === 'active')
  const usableGrant = activeApp
    ? selectUsableMobileCaptureGrant(grants, merchant.merchant.id, activeApp.app_id)
    : undefined
  const runtimeReady = runtimeBinding?.status === 'active'

  const loadApps = useCallback(async () => {
    setLoadingApps(true)
    try {
      const response = await openCommerceClientApi.listApps(projectId)
      setApps(response.apps)
    } catch (loadError) {
      setError(true)
      setMessage(errorMessage(loadError))
    } finally {
      setLoadingApps(false)
    }
  }, [projectId])

  useEffect(() => {
    void loadApps()
  }, [loadApps])

  useEffect(() => {
    if (!launch) return undefined
    const delay = Math.max(0, launch.expiresAtUnix * 1000 - Date.now())
    const timer = window.setTimeout(() => {
      setLaunch(null)
      setMessage('上一个一次性入口已过期，请重新生成。')
    }, delay)
    return () => window.clearTimeout(timer)
  }, [launch])

  async function createLaunch() {
    if (busy || loadingApps) return
    setBusy(true)
    setError(false)
    setMessage('')
    setLaunch(null)
    let appId = activeApp?.app_id
    let confirmationId: string | undefined
    try {
      if (!runtimeReady) throw new Error('请先保存并验证该商户的运行时绑定。')
      if (!userId) throw new Error('当前一龙账号身份不可用，请重新登录。')

      let selectedCapability = capability
      if (!selectedCapability) {
        if (!canEdit) throw new Error('当前角色不能登记手机平台账号能力。')
        selectedCapability = await openCommerceApi.createCapability(
          projectId,
          merchant.merchant.id,
          mobileCaptureCapabilityDefinition(),
        )
      }
      assertCompatibleMobileCaptureCapability(selectedCapability)

      let selectedApp = activeApp
      if (!selectedApp) {
        if (!canEdit) throw new Error('当前角色没有可用的手机绑定应用身份。')
        const disabledApp = ownedApps.find((app) => app.status === 'disabled')
        const credential = disabledApp
          ? await openCommerceClientApi.reactivateApp(projectId, disabledApp.id)
          : await openCommerceClientApi.createApp(projectId, {
            app_id: `merchant-mobile-${crypto.randomUUID()}`,
            display_name: APP_DISPLAY_NAME,
          })
        selectedApp = credential.app
        appId = selectedApp.app_id
      }
      appId = selectedApp.app_id

      let selectedGrant = selectUsableMobileCaptureGrant(
        grants,
        merchant.merchant.id,
        selectedApp.app_id,
      )
      if (!selectedGrant) {
        if (!canEdit) throw new Error('当前应用没有可用的手机绑定授权。')
        selectedGrant = await openCommerceApi.createGrant(projectId, {
          merchant_id: merchant.merchant.id,
          grantee_app_id: selectedApp.app_id,
          scopes: [MOBILE_CAPTURE_CAPABILITY],
          purpose: '允许商户本人为自己的手机签发平台账号绑定入口',
          expires_at: expiresInThirtyDays(),
          max_invocations: 100,
          budget_currency: 'CNY',
        })
      }

      const request = {
        merchant_id: merchant.merchant.id,
        capability_key: MOBILE_CAPTURE_CAPABILITY,
        requester_app_id: selectedApp.app_id,
        grant_id: selectedGrant.id,
        idempotency_key: `merchant-mobile-capture-${crypto.randomUUID()}`,
        input: { target: MOBILE_CAPTURE_TARGET },
      }
      const prepared = await openCommerceClientApi.prepareActionConfirmation(
        selectedApp.app_id,
        request,
      )
      confirmationId = prepared.id
      const confirmed = await openCommerceClientApi.confirmActionConfirmation(
        selectedApp.app_id,
        prepared.id,
      )
      const invocation = await openCommerceClientApi.invokeAsApp(selectedApp.app_id, {
        ...request,
        action_confirmation_id: confirmed.id,
      })
      setLaunch(parseMobileCaptureInvocation(invocation))
      setMessage('一次性入口已生成。请在 2 分钟内从安卓手机打开。')
      await Promise.all([loadApps(), onChanged()])
    } catch (launchError) {
      if (confirmationId && appId) {
        await openCommerceClientApi.cancelActionConfirmation(appId, confirmationId).catch(() => undefined)
      }
      setError(true)
      setMessage(errorMessage(launchError))
    } finally {
      setBusy(false)
    }
  }

  async function copyLaunchLink() {
    if (!launch) return
    try {
      await navigator.clipboard.writeText(launch.launchUrl)
      setError(false)
      setMessage('一次性入口已复制；请只发送到本人的安卓手机。')
    } catch (copyError) {
      setError(true)
      setMessage(errorMessage(copyError))
    }
  }

  async function shareLaunchLink() {
    if (!launch || !navigator.share) return
    try {
      await navigator.share({
        title: '手机商户平台账号绑定',
        text: '请在本人安卓手机上打开此一次性绑定入口。',
        url: launch.launchUrl,
      })
    } catch (shareError) {
      if (shareError instanceof DOMException && shareError.name === 'AbortError') return
      setError(true)
      setMessage(errorMessage(shareError))
    }
  }

  const setupReady = Boolean(capability && activeApp && usableGrant)

  return (
    <section className={styles.panel} aria-labelledby="mobile-capture-title">
      <header className={styles.header}>
        <span className={styles.icon} aria-hidden="true"><Smartphone size={20} /></span>
        <span className={styles.title}>
          <strong id="mobile-capture-title">手机平台账号</strong>
          <small>美团外卖、淘宝闪购、京东到家</small>
        </span>
        <span className={runtimeReady ? styles.ready : styles.pending}>
          {runtimeReady ? <CheckCircle2 size={14} /> : <RefreshCw size={14} />}
          {runtimeReady ? '运行时已验证' : '等待运行时'}
        </span>
      </header>

      <div className={styles.body}>
        <div className={styles.summary}>
          <strong>{setupReady ? '绑定通道已准备' : '首次使用会自动准备最小授权'}</strong>
          <p>
            平台密码和 Cookie 只保存在商户本人的手机 WebView；一龙只签发单次入口，
            商户服务器只接收白名单订单响应。
          </p>
          <span><ShieldCheck size={14} />固定包名 · HTTPS 服务 · 单次票据 · 本人应用身份</span>
        </div>

        <div className={styles.actions}>
          <button
            className={styles.primary}
            type="button"
            onClick={() => void createLaunch()}
            disabled={busy || loadingApps || !runtimeReady || (!canEdit && !setupReady)}
          >
            {busy || loadingApps
              ? <LoaderCircle className={styles.spin} size={17} />
              : <Smartphone size={17} />}
            {busy ? '正在签发' : loadingApps ? '正在检查' : '生成手机绑定入口'}
          </button>
          {!runtimeReady && <small>先在本页下方完成商户应用运行时配置和签名验证。</small>}
        </div>
      </div>

      {launch && (
        <div className={styles.launchBar}>
          <span>
            <strong>入口已就绪</strong>
            <small>有效至 {new Date(launch.expiresAtUnix * 1000).toLocaleTimeString('zh-CN')}</small>
          </span>
          <a className={styles.openButton} href={launch.androidIntentUrl}>
            <ExternalLink size={16} />在安卓手机打开
          </a>
          {typeof navigator.share === 'function' && (
            <button type="button" onClick={() => void shareLaunchLink()} title="分享到本人手机">
              <Share2 size={16} /><span>分享</span>
            </button>
          )}
          <button type="button" onClick={() => void copyLaunchLink()} title="复制一次性入口">
            <Copy size={16} /><span>复制</span>
          </button>
        </div>
      )}

      {message && <div className={styles.message} data-error={error}>{message}</div>}
    </section>
  )
}

function expiresInThirtyDays() {
  return new Date(Date.now() + 30 * 24 * 60 * 60 * 1000).toISOString()
}

function errorMessage(error: unknown) {
  return error instanceof Error && error.message ? error.message : '手机平台账号绑定失败。'
}

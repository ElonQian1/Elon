import { useCallback, useEffect, useState } from 'react'
import { Bot, CheckCircle2, Link2, RefreshCw, Store, Unlink, XCircle } from 'lucide-react'
import { erpBlueprintApi } from './erpBlueprintApi'
import type { ErpInstance, ErpOpenCommerceReadiness } from './erpBlueprintTypes'
import { errorMessage } from './erpBlueprintUi'
import styles from './ErpBlueprintPanel.module.css'

export default function ErpOpenCommerceReadinessPanel({
  projectId,
  instance,
  canEdit,
  refresh,
}: {
  projectId: string
  instance: ErpInstance
  canEdit: boolean
  refresh: () => Promise<void>
}) {
  const [readiness, setReadiness] = useState<ErpOpenCommerceReadiness | null>(null)
  const [merchantId, setMerchantId] = useState('')
  const [loading, setLoading] = useState(true)
  const [message, setMessage] = useState('')

  const load = useCallback(async (selectedMerchantId?: string) => {
    setLoading(true)
    setMessage('')
    try {
      const next = await erpBlueprintApi.openCommerceReadiness(
        projectId,
        instance.id,
        selectedMerchantId || undefined,
      )
      setReadiness(next)
      setMerchantId(next.merchant_selection.selected?.id ?? '')
    } catch (error) {
      setMessage(errorMessage(error))
    } finally {
      setLoading(false)
    }
  }, [instance.id, instance.open_commerce_merchant_id, projectId])

  useEffect(() => {
    void load()
  }, [load])

  async function updateBinding(nextMerchantId: string | null) {
    if (!window.confirm(nextMerchantId ? '确认将当前 ERP 绑定到所选商户节点？' : '确认解除当前 ERP 的商户节点绑定？')) return
    setLoading(true)
    setMessage('')
    try {
      await erpBlueprintApi.updateOpenCommerceMerchant(projectId, instance.id, {
        expected_revision: instance.configuration_revision,
        merchant_confirmed: true,
        merchant_id: nextMerchantId,
      })
      setMessage(nextMerchantId ? '开放商业商户节点已绑定。' : '开放商业商户节点绑定已解除。')
      await refresh()
    } catch (error) {
      setMessage(errorMessage(error))
    } finally {
      setLoading(false)
    }
  }

  return (
    <section className={styles.band}>
      <header>
        <Bot size={17} />
        <div>
          <h3>消费者 AI 接入就绪度</h3>
          <p className={styles.mutedLine}>状态来自 ERP、商户节点、运行时、能力目录和物化任务的现有记录。</p>
        </div>
        <button
          type="button"
          className={styles.iconButton}
          disabled={loading}
          onClick={() => void load(merchantId)}
          title="刷新开放商业就绪度"
          aria-label="刷新开放商业就绪度"
        >
          <RefreshCw size={15} className={loading ? styles.spin : undefined} />
        </button>
      </header>

      {readiness && (readiness.merchant_selection.candidates.length > 0 || instance.open_commerce_merchant_id) && (
        <div className={styles.readinessBindingRow}>
          <label className={styles.readinessMerchantSelect}>
            <span>ERP 对应商户节点</span>
            <select
              value={merchantId}
              disabled={!!instance.open_commerce_merchant_id}
              onChange={(event) => {
                setMerchantId(event.target.value)
                if (event.target.value) void load(event.target.value)
              }}
            >
              <option value="">请选择</option>
              {instance.open_commerce_merchant_id
                && !readiness.merchant_selection.candidates.some((merchant) => merchant.id === instance.open_commerce_merchant_id)
                && <option value={instance.open_commerce_merchant_id}>已失效绑定</option>}
              {readiness.merchant_selection.candidates.map((merchant) => (
                <option key={merchant.id} value={merchant.id}>{merchant.display_name}</option>
              ))}
            </select>
          </label>
          {instance.open_commerce_merchant_id ? (
            <button
              type="button"
              className={styles.iconButton}
              disabled={!canEdit || loading}
              onClick={() => void updateBinding(null)}
              title="解除 ERP 商户节点绑定"
              aria-label="解除 ERP 商户节点绑定"
            ><Unlink size={15} /></button>
          ) : (
            <button
              type="button"
              className={styles.iconButton}
              disabled={!canEdit || loading || !merchantId}
              onClick={() => void updateBinding(merchantId)}
              title="绑定为当前 ERP 的商户节点"
              aria-label="绑定为当前 ERP 的商户节点"
            ><Link2 size={15} /></button>
          )}
        </div>
      )}

      {readiness && (
        <>
          <div className={styles.readinessGates}>
            <ReadinessGate label="ERP 项目验收" ready={readiness.erp_onboarding_ready} detail={readiness.materialization.state} />
            <ReadinessGate label="消费者 AI 调用" ready={readiness.consumer_invocation_ready} detail={`${readiness.active_runtime_capability_keys.length} 项运行时能力`} />
            <ReadinessGate label="开放目录发现" ready={readiness.consumer_discovery_ready} detail={readiness.directory?.status ?? '未发布'} />
          </div>
          {readiness.merchant_selection.selected && (
            <p className={styles.selectedMerchantLine}>
              <Store size={14} />{readiness.merchant_selection.selected.display_name}
              <span>{readiness.merchant_selection.status === 'selected_binding' ? '已稳定绑定' : '预览选择'} · {readiness.runtime?.status ?? '未配置运行时'}</span>
            </p>
          )}
          {!!readiness.blockers.length && (
            <div className={styles.readinessBlockers}>
              {readiness.blockers.map((blocker) => (
                <div key={`${blocker.scope}:${blocker.code}`}>
                  <strong>{blocker.message}</strong>
                  <span>{blocker.next_action}</span>
                </div>
              ))}
            </div>
          )}
        </>
      )}
      {message && <p className={styles.message}>{message}</p>}
    </section>
  )
}

function ReadinessGate({ label, ready, detail }: { label: string; ready: boolean; detail: string }) {
  const Icon = ready ? CheckCircle2 : XCircle
  return (
    <div data-ready={ready}>
      <Icon size={16} />
      <span>{label}</span>
      <strong>{ready ? '已就绪' : '未就绪'}</strong>
      <small>{detail}</small>
    </div>
  )
}

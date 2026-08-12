import { useCallback, useEffect, useState } from 'react'
import { Bot, CheckCircle2, RefreshCw, Store, XCircle } from 'lucide-react'
import { erpBlueprintApi } from './erpBlueprintApi'
import type { ErpOpenCommerceReadiness } from './erpBlueprintTypes'
import { errorMessage } from './erpBlueprintUi'
import styles from './ErpBlueprintPanel.module.css'

export default function ErpOpenCommerceReadinessPanel({
  projectId,
  instanceId,
}: {
  projectId: string
  instanceId: string
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
        instanceId,
        selectedMerchantId || undefined,
      )
      setReadiness(next)
      if (next.merchant_selection.selected) setMerchantId(next.merchant_selection.selected.id)
    } catch (error) {
      setMessage(errorMessage(error))
    } finally {
      setLoading(false)
    }
  }, [instanceId, projectId])

  useEffect(() => {
    void load()
  }, [load])

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

      {readiness && readiness.merchant_selection.candidates.length > 1 && (
        <label className={styles.readinessMerchantSelect}>
          <span>商户节点</span>
          <select
            value={merchantId}
            onChange={(event) => {
              setMerchantId(event.target.value)
              void load(event.target.value)
            }}
          >
            <option value="">请选择</option>
            {readiness.merchant_selection.candidates.map((merchant) => (
              <option key={merchant.id} value={merchant.id}>{merchant.display_name}</option>
            ))}
          </select>
        </label>
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
              <span>{readiness.runtime?.status ?? '未配置运行时'}</span>
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

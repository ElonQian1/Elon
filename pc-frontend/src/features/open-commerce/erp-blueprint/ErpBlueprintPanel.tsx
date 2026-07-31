import { useCallback, useEffect, useState } from 'react'
import { Boxes, RefreshCw, Shield } from 'lucide-react'
import BlueprintMaintainerView from './BlueprintMaintainerView'
import BlueprintSetupForm from './BlueprintSetupForm'
import { erpBlueprintApi } from './erpBlueprintApi'
import type { ErpOverview } from './erpBlueprintTypes'
import { errorMessage } from './erpBlueprintUi'
import ErpInstanceView from './ErpInstanceView'
import styles from './ErpBlueprintPanel.module.css'

export default function ErpBlueprintPanel({
  projectId,
  canEdit,
}: {
  projectId: string
  canEdit: boolean
}) {
  const [overview, setOverview] = useState<ErpOverview | null>(null)
  const [loading, setLoading] = useState(true)
  const [message, setMessage] = useState('')

  const refresh = useCallback(async () => {
    setLoading(true)
    setMessage('')
    try {
      setOverview(await erpBlueprintApi.overview(projectId))
    } catch (error) {
      setMessage(errorMessage(error))
    } finally {
      setLoading(false)
    }
  }, [projectId])

  useEffect(() => {
    refresh()
  }, [refresh])

  if (loading && !overview) return <div className={styles.loading}>正在读取 ERP 蓝图…</div>

  const isMaintainer = overview?.blueprint?.definition.source_project_id === projectId
  return (
    <div className={styles.panel}>
      <section className={styles.hero}>
        <div>
          <span><Shield size={13} /> ERP BLUEPRINT V1</span>
          <h2>同一套稳定内核，每个商户拥有独立项目</h2>
          <p>AI 先复用能力，再开发私有扩展；共性需求经脱敏聚合和人工评审后才进入公共内核。</p>
        </div>
        <button type="button" title="刷新" onClick={refresh} disabled={loading}>
          <RefreshCw size={16} className={loading ? styles.spin : undefined} />
        </button>
      </section>

      {!overview?.blueprint && (
        <BlueprintSetupForm projectId={projectId} canEdit={canEdit} onCreated={refresh} />
      )}
      {overview?.blueprint && isMaintainer && (
        <BlueprintMaintainerView projectId={projectId} canEdit={canEdit} overview={overview} refresh={refresh} />
      )}
      {overview?.blueprint && overview.instance && !isMaintainer && (
        <ErpInstanceView projectId={projectId} canEdit={canEdit} overview={overview} refresh={refresh} />
      )}
      {overview?.blueprint && !overview.instance && !isMaintainer && (
        <div className={styles.emptyState}><Boxes size={24} /><p>当前项目已关联蓝图，但不是可管理的商户实例。</p></div>
      )}
      {message && <p className={styles.message}>{message}</p>}
    </div>
  )
}

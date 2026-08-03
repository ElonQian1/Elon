import { useCallback, useEffect, useMemo, useState } from 'react'
import { Download, PackagePlus, Plus, RefreshCw, ShieldCheck } from 'lucide-react'
import { taskEconomyApi } from './taskEconomyApi'
import { downloadSuiAdapterHandoff } from './suiAdapterHandoffDownload'
import type {
  SuiCorrectionProjectionPackage,
  SuiTargetNetwork,
} from './taskEconomyTypes'
import { errorText } from './openCommerceUi'
import {
  actionStyle,
  badgeStyle,
  commerceStyles,
  errorMessageStyle,
} from './openCommerceStyles'

export default function SuiCorrectionProjectionPackages({
  projectId,
  correctionId,
  canEdit,
}: {
  projectId: string
  correctionId: string
  canEdit: boolean
}) {
  const [packages, setPackages] = useState<SuiCorrectionProjectionPackage[]>([])
  const [targetNetwork, setTargetNetwork] = useState<SuiTargetNetwork>('testnet')
  const [busy, setBusy] = useState('')
  const [message, setMessage] = useState('')

  const refresh = useCallback(async () => {
    setMessage('')
    try {
      setPackages(await taskEconomyApi.suiCorrectionProjections(projectId))
    } catch (error) {
      setMessage(errorText(error))
    }
  }, [projectId])

  useEffect(() => {
    refresh()
  }, [refresh])

  const items = useMemo(
    () => packages.filter((item) => item.correction_id === correctionId),
    [correctionId, packages],
  )

  async function prepare() {
    setBusy('prepare')
    setMessage('')
    try {
      await taskEconomyApi.prepareSuiCorrectionProjection(
        projectId,
        correctionId,
        targetNetwork,
      )
      await refresh()
    } catch (error) {
      setMessage(errorText(error))
    } finally {
      setBusy('')
    }
  }

  async function verify(item: SuiCorrectionProjectionPackage) {
    setBusy(item.id)
    setMessage('')
    try {
      await taskEconomyApi.verifySuiCorrectionProjection(projectId, item.id)
      await refresh()
    } catch (error) {
      setMessage(errorText(error))
    } finally {
      setBusy('')
    }
  }

  async function downloadHandoff(item: SuiCorrectionProjectionPackage) {
    setBusy(`handoff:${item.id}`)
    setMessage('')
    try {
      const bundle = await taskEconomyApi.suiCorrectionProjectionAdapterHandoff(
        projectId,
        item.id,
      )
      downloadSuiAdapterHandoff(bundle)
      await refresh()
    } catch (error) {
      setMessage(errorText(error))
    } finally {
      setBusy('')
    }
  }

  async function queuePreflight(item: SuiCorrectionProjectionPackage) {
    setBusy(`queue:${item.id}`)
    setMessage('')
    try {
      await taskEconomyApi.queueSuiPreflightJob(projectId, 'correction', item.id)
      setMessage('原子纠正包已加入离线预检队列。')
    } catch (error) {
      setMessage(errorText(error))
    } finally {
      setBusy('')
    }
  }

  return (
    <div style={commerceStyles.list}>
      <header style={commerceStyles.itemHeader}>
        <div>
          <strong>Sui 原子纠正包</strong>
          <small>冲销与替换必须一起复核，当前不提交网络</small>
        </div>
        <button style={actionStyle('icon')} type="button" onClick={refresh} title="刷新纠正投影包">
          <RefreshCw size={14} />
        </button>
      </header>
      <div style={commerceStyles.headerActions}>
        <select
          aria-label="Sui 纠正包目标网络"
          value={targetNetwork}
          onChange={(event) => setTargetNetwork(event.target.value as SuiTargetNetwork)}
          disabled={busy !== ''}
        >
          <option value="devnet">Devnet</option>
          <option value="testnet">Testnet</option>
          <option value="mainnet">Mainnet</option>
        </select>
        <button
          style={actionStyle('secondary', !canEdit || busy !== '')}
          type="button"
          onClick={prepare}
          disabled={!canEdit || busy !== ''}
        >
          <PackagePlus size={14} />{busy === 'prepare' ? '保存中' : '保存原子包'}
        </button>
      </div>
      {items.map((item) => (
        <div style={commerceStyles.priorityRow} key={item.id}>
          <span style={badgeStyle(item.integrity_status === 'verified' ? 'neutral' : 'danger')}>
            {item.target_network} · {item.submission_readiness}
          </span>
          <code>{item.projection_digest.slice(0, 20)}</code>
          <div style={commerceStyles.headerActions}>
            <button
              style={actionStyle('icon')}
              type="button"
              onClick={() => verify(item)}
              disabled={!canEdit || busy !== ''}
              title="复核两条腿和摘要"
            >
              <ShieldCheck size={14} />
            </button>
            <button
              style={actionStyle('icon')}
              type="button"
              onClick={() => downloadHandoff(item)}
              disabled={
                !canEdit || busy !== '' || item.submission_readiness !== 'adapter_required'
              }
              title="重新复核并下载离线适配器交接包"
            >
              <Download size={14} />
            </button>
            <button
              style={actionStyle('icon')}
              type="button"
              onClick={() => queuePreflight(item)}
              disabled={
                !canEdit || busy !== '' || item.submission_readiness !== 'adapter_required'
              }
              title="把原子纠正包加入离线预检任务队列"
            >
              <Plus size={14} />
            </button>
          </div>
        </div>
      ))}
      {items.length === 0 && <small style={commerceStyles.itemMeta}>尚未保存任何目标网络的原子纠正包。</small>}
      <small style={commerceStyles.itemMeta}>NOT SUBMITTED · 0 次网络提交 · 不移动真实资金</small>
      {message && <div style={{ ...commerceStyles.message, ...errorMessageStyle }}>{message}</div>}
    </div>
  )
}

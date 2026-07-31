import { useMemo, useState } from 'react'
import { ArrowUpCircle, CheckCircle2, RotateCcw } from 'lucide-react'
import { erpBlueprintApi } from './erpBlueprintApi'
import type { ErpOverview, ErpUpgrade } from './erpBlueprintTypes'
import { errorMessage, isNewerVersion, shortDate } from './erpBlueprintUi'
import styles from './ErpBlueprintPanel.module.css'

export default function ErpUpgradePanel({
  projectId,
  canEdit,
  overview,
  refresh,
}: {
  projectId: string
  canEdit: boolean
  overview: ErpOverview
  refresh: () => Promise<void>
}) {
  const instance = overview.instance!
  const [targetVersion, setTargetVersion] = useState('')
  const [busy, setBusy] = useState(false)
  const [message, setMessage] = useState('')
  const availableVersions = useMemo(
    () => overview.versions.filter((item) => isNewerVersion(item.manifest.version, instance.pinned_version)),
    [instance.pinned_version, overview.versions],
  )

  async function run(action: () => Promise<unknown>, success: string) {
    setBusy(true)
    setMessage('')
    try {
      await action()
      setMessage(success)
      await refresh()
    } catch (error) {
      setMessage(errorMessage(error))
    } finally {
      setBusy(false)
    }
  }

  return (
    <section className={styles.band}>
      <header><ArrowUpCircle size={17} /><h3>版本升级</h3></header>
      <div className={styles.inlineForm}>
        <label className={styles.grow}>目标版本<select value={targetVersion} onChange={(event) => setTargetVersion(event.target.value)}><option value="">选择版本</option>{availableVersions.map((item) => <option key={item.id} value={item.manifest.version}>{item.manifest.version}</option>)}</select></label>
        <button type="button" disabled={!canEdit || busy || !targetVersion} onClick={() => run(
          () => erpBlueprintApi.prepareUpgrade(projectId, instance.id, targetVersion),
          '兼容检查已完成，实例配置尚未改变。',
        )}><CheckCircle2 size={15} />检查兼容性</button>
      </div>
      <div className={styles.upgradeList}>
        {overview.upgrades.map((upgrade) => (
          <UpgradeRow key={upgrade.id} projectId={projectId} canEdit={canEdit} busy={busy} upgrade={upgrade} run={run} />
        ))}
        {!overview.upgrades.length && <p className={styles.empty}>尚无升级活动。</p>}
      </div>
      {message && <p className={styles.message}>{message}</p>}
    </section>
  )
}

function UpgradeRow({
  projectId,
  canEdit,
  busy,
  upgrade,
  run,
}: {
  projectId: string
  canEdit: boolean
  busy: boolean
  upgrade: ErpUpgrade
  run: (action: () => Promise<unknown>, success: string) => Promise<void>
}) {
  const [confirmed, setConfirmed] = useState(false)
  const [summary, setSummary] = useState('')
  const [commit, setCommit] = useState('')
  const [rollbackReason, setRollbackReason] = useState('')
  return (
    <article className={styles.upgrade} data-status={upgrade.status}>
      <div className={styles.upgradeSummary}>
        <strong>{upgrade.compatibility.from_version} → {upgrade.compatibility.target_version}</strong>
        <span>{upgrade.status} · {shortDate(upgrade.updated_at)} · 配置修订 {upgrade.instance_revision}</span>
        {upgrade.compatibility.issues.map((issue) => <p key={`${issue.code}-${issue.subject}`}>{issue.message}</p>)}
        {!upgrade.compatibility.issues.length && <p>模块、插件与私有扩展边界通过检查。</p>}
        <p>模块：{upgrade.from_configuration.enabled_modules.join('、')} → {upgrade.target_configuration.enabled_modules.join('、')}</p>
        {upgrade.adoption_evidence && <p>验证：{upgrade.adoption_evidence.verification_summary}{upgrade.adoption_evidence.deployed_commit ? ` · ${upgrade.adoption_evidence.deployed_commit}` : ''}</p>}
      </div>
      {upgrade.status === 'ready' && (
        <div className={styles.upgradeDecision}>
          <label className={styles.checkLabel}><input type="checkbox" checked={confirmed} onChange={(event) => setConfirmed(event.target.checked)} />商户确认已完成开发、迁移和验证</label>
          <textarea value={summary} onChange={(event) => setSummary(event.target.value)} placeholder="验证摘要（8 至 500 字）" />
          <input value={commit} onChange={(event) => setCommit(event.target.value)} placeholder="已部署 Git 提交（可选）" />
          <button type="button" disabled={!canEdit || busy || !confirmed || summary.trim().length < 8} onClick={() => run(
            () => erpBlueprintApi.decideUpgrade(projectId, upgrade.id, {
              action: 'adopt',
              reason: '',
              merchant_confirmed: true,
              execution_attested: true,
              verification_summary: summary,
              deployed_commit: commit.trim() || null,
            }),
            '目标版本和配置快照已采用，验证证据已记录。',
          )}><CheckCircle2 size={15} />确认采用</button>
        </div>
      )}
      {upgrade.status === 'adopted' && (
        <div className={styles.upgradeDecision}>
          <input value={rollbackReason} onChange={(event) => setRollbackReason(event.target.value)} placeholder="回滚原因" />
          <button type="button" disabled={!canEdit || busy || !rollbackReason.trim()} onClick={() => run(
            () => erpBlueprintApi.decideUpgrade(projectId, upgrade.id, {
              action: 'rollback',
              reason: rollbackReason,
              merchant_confirmed: true,
              execution_attested: false,
              verification_summary: '',
              deployed_commit: null,
            }),
            '已恢复升级前版本、主题、模块和插件配置。',
          )}><RotateCcw size={15} />回滚版本</button>
        </div>
      )}
    </article>
  )
}

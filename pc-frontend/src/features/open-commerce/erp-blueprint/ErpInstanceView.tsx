import { useMemo, useState } from 'react'
import { ArrowUpCircle, CheckCircle2, RotateCcw, Search, Send } from 'lucide-react'
import { erpBlueprintApi } from './erpBlueprintApi'
import type { ErpOverview, RequirementResolution } from './erpBlueprintTypes'
import { classificationLabels, errorMessage, isNewerVersion, shortDate } from './erpBlueprintUi'
import styles from './ErpBlueprintPanel.module.css'

export default function ErpInstanceView({
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
  const [requirement, setRequirement] = useState('')
  const [scope, setScope] = useState<'merchant_specific' | 'potential_common'>('merchant_specific')
  const [resolution, setResolution] = useState<RequirementResolution | null>(null)
  const [authorized, setAuthorized] = useState(false)
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

  async function resolve() {
    setBusy(true)
    setMessage('')
    try {
      const next = await erpBlueprintApi.resolveRequirement(projectId, {
        instance_id: instance.id,
        requirement,
        expected_scope: scope,
      })
      setResolution(next)
    } catch (error) {
      setMessage(errorMessage(error))
    } finally {
      setBusy(false)
    }
  }

  return (
    <div className={styles.workspace}>
      <section className={styles.instanceHeader}>
        <div>
          <span>独立商户 ERP</span>
          <h3>{instance.instance_key}</h3>
          <p>{instance.industry} · {instance.theme_key} · 蓝图 v{instance.pinned_version}</p>
        </div>
        <div className={styles.extensionCount}>
          <strong>{instance.private_extensions.length}</strong>
          <span>私有扩展受保护</span>
        </div>
      </section>

      <section className={styles.band}>
        <header><Search size={17} /><h3>让 AI 先查能力，再决定怎么开发</h3></header>
        <textarea
          className={styles.requirementInput}
          value={requirement}
          onChange={(event) => setRequirement(event.target.value)}
          placeholder="例如：根据保质期和销量自动生成补货建议"
        />
        <div className={styles.segmented}>
          <button type="button" data-active={scope === 'merchant_specific'} onClick={() => setScope('merchant_specific')}>本店专有</button>
          <button type="button" data-active={scope === 'potential_common'} onClick={() => setScope('potential_common')}>可能通用</button>
        </div>
        <button type="button" disabled={busy || requirement.trim().length < 4} onClick={resolve}>
          <Search size={15} />分析需求
        </button>
        {resolution && (
          <div className={styles.resolution} data-kind={resolution.classification}>
            <strong>{classificationLabels[resolution.classification] ?? resolution.classification}</strong>
            <p>{resolution.recommendation}</p>
            {!!resolution.matched_capabilities.length && (
              <div className={styles.chips}>{resolution.matched_capabilities.map((item) => <span key={item.capability_key}>{item.display_name}</span>)}</div>
            )}
            {resolution.may_submit_signal && (
              <div className={styles.signalBox}>
                <label className={styles.checkLabel}>
                  <input type="checkbox" checked={authorized} onChange={(event) => setAuthorized(event.target.checked)} />
                  商户确认只提交脱敏需求摘要，不含客户数据、密钥或私有源码
                </label>
                <button
                  type="button"
                  disabled={!canEdit || busy || !authorized}
                  onClick={() => run(
                    () => erpBlueprintApi.submitSignal(projectId, instance.id, {
                      schema: 'yilong.erp.feature_signal.v1',
                      requirement_summary: resolution.requirement,
                      need_key: resolution.need_key,
                      industry: instance.industry,
                      requested_outcome: resolution.requirement,
                      merchant_authorized: true,
                      classification: 'sanitized_aggregate',
                      evidence: { occurrence_count: 1 },
                    }),
                    '脱敏需求信号已提交；维护者仍需人工决定是否进入公共内核。',
                  )}
                ><Send size={15} />授权提交</button>
              </div>
            )}
          </div>
        )}
      </section>

      <section className={styles.band}>
        <header><ArrowUpCircle size={17} /><h3>版本升级</h3></header>
        <div className={styles.inlineForm}>
          <label className={styles.grow}>
            目标版本
            <select value={targetVersion} onChange={(event) => setTargetVersion(event.target.value)}>
              <option value="">选择版本</option>
              {availableVersions.map((item) => <option key={item.id} value={item.manifest.version}>{item.manifest.version}</option>)}
            </select>
          </label>
          <button
            type="button"
            disabled={!canEdit || busy || !targetVersion}
            onClick={() => run(
              () => erpBlueprintApi.prepareUpgrade(projectId, instance.id, targetVersion),
              '兼容检查已完成；平台尚未修改代码或部署。',
            )}
          ><CheckCircle2 size={15} />检查兼容性</button>
        </div>
        <div className={styles.upgradeList}>
          {overview.upgrades.map((upgrade) => (
            <article key={upgrade.id} className={styles.upgrade} data-status={upgrade.status}>
              <div>
                <strong>{upgrade.compatibility.from_version} → {upgrade.compatibility.target_version}</strong>
                <span>{upgrade.status} · {shortDate(upgrade.updated_at)}</span>
                {upgrade.compatibility.issues.map((issue) => <p key={`${issue.code}-${issue.subject}`}>{issue.message}</p>)}
                {!upgrade.compatibility.issues.length && <p>模块、插件与私有扩展边界通过检查。</p>}
              </div>
              {upgrade.status === 'ready' && (
                <button type="button" disabled={!canEdit || busy} onClick={() => run(
                  () => erpBlueprintApi.decideUpgrade(projectId, upgrade.id, 'adopt'),
                  '已记录采用目标版本；真实代码发布仍由项目发布流程完成。',
                )}><CheckCircle2 size={15} />确认采用</button>
              )}
              {upgrade.status === 'adopted' && (
                <button type="button" disabled={!canEdit || busy} onClick={() => run(
                  () => erpBlueprintApi.decideUpgrade(projectId, upgrade.id, 'rollback', '商户人工回滚'),
                  '已恢复升级前固定版本，私有扩展清单保持不变。',
                )}><RotateCcw size={15} />回滚版本</button>
              )}
            </article>
          ))}
          {!overview.upgrades.length && <p className={styles.empty}>尚无升级活动。</p>}
        </div>
      </section>

      <section className={styles.band}>
        <header><CheckCircle2 size={17} /><h3>当前可复用能力</h3></header>
        <div className={styles.capabilityGrid}>
          {overview.capability_catalog.map((item) => (
            <div key={item.capability_key}>
              <strong>{item.display_name}</strong>
              <span>{item.category} · {item.capability_key}</span>
            </div>
          ))}
        </div>
      </section>
      {message && <p className={styles.message}>{message}</p>}
    </div>
  )
}

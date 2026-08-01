import { useEffect, useMemo, useState } from 'react'
import { Check, FolderInput, GitBranch, PackagePlus, Plus, RotateCcw, X } from 'lucide-react'
import { erpBlueprintApi } from './erpBlueprintApi'
import BlueprintEvolutionForm from './BlueprintEvolutionForm'
import type { ErpOverview, ErpReleaseManifest, ErpTargetProject } from './erpBlueprintTypes'
import { errorMessage, shortDate } from './erpBlueprintUi'
import styles from './ErpBlueprintPanel.module.css'

export default function BlueprintMaintainerView({
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
  const blueprint = overview.blueprint!
  const [version, setVersion] = useState('1.0.0')
  const [commit, setCommit] = useState('')
  const [instanceName, setInstanceName] = useState('')
  const [instanceKey, setInstanceKey] = useState('')
  const [onboardingMode, setOnboardingMode] = useState<'new_project' | 'existing_project'>('new_project')
  const [targetProjectId, setTargetProjectId] = useState('')
  const [targetProjects, setTargetProjects] = useState<ErpTargetProject[]>([])
  const [industry, setIndustry] = useState('local_retail')
  const [theme, setTheme] = useState(blueprint.definition.themes[0] ?? 'default.clean')
  const [targetVersion, setTargetVersion] = useState(overview.versions[0]?.manifest.version ?? '')
  const [busy, setBusy] = useState(false)
  const [message, setMessage] = useState('')
  const latest = overview.versions[0]
  const versionOptions = useMemo(
    () => overview.versions.map((item) => item.manifest.version),
    [overview.versions],
  )
  const eligibleTargetProjects = useMemo(() => {
    const boundProjectIds = new Set(overview.instances.map((instance) => instance.project_id))
    return targetProjects.filter((project) => (
      project.id !== projectId
      && !boundProjectIds.has(project.id)
      && ['owner', 'admin', 'editor'].includes(
        project.viewer_role ?? project.role ?? project.my_role ?? '',
      )
    ))
  }, [overview.instances, projectId, targetProjects])

  useEffect(() => {
    let active = true
    erpBlueprintApi.listTargetProjects()
      .then((response) => {
        if (active) setTargetProjects(response.projects ?? [])
      })
      .catch(() => {
        if (active) setTargetProjects([])
      })
    return () => { active = false }
  }, [])

  function selectOnboardingMode(mode: 'new_project' | 'existing_project') {
    setOnboardingMode(mode)
    if (mode === 'existing_project' && !targetProjectId) {
      setTargetProjectId(eligibleTargetProjects[0]?.id ?? '')
    }
  }

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

  function manifest(): ErpReleaseManifest {
    return {
      schema: 'yilong.erp.release.v1',
      blueprint_key: blueprint.definition.blueprint_key,
      version,
      previous_version: latest?.manifest.version ?? null,
      source_git_commit: commit.trim(),
      modules: blueprint.definition.modules.map(({ module_key, required }) => ({
        module_key,
        version,
        required,
      })),
      capabilities: blueprint.definition.capabilities.map(({ capability_key }) => capability_key),
      extension_points: blueprint.definition.extension_points,
      migrations: [],
      compatibility: {
        minimum_instance_version: latest?.manifest.version ?? version,
        required_plugins: [],
      },
      rollback: {
        supported: true,
        instructions: '恢复升级前固定版本，并使用既有项目发布流程验证数据库与运行状态。',
      },
    }
  }

  return (
    <div className={styles.workspace}>
      <section className={styles.metrics}>
        <Metric label="已发布版本" value={String(overview.versions.length)} />
        <Metric label="商户实例" value={String(overview.instances.length)} />
        <Metric label="能力目录" value={String(overview.capability_catalog.length)} />
        <Metric label="待审提案" value={String(overview.proposals.filter((item) => item.status === 'candidate').length)} />
      </section>

      <section className={styles.band}>
        <header><GitBranch size={17} /><h3>发布不可变蓝图版本</h3></header>
        <div className={styles.inlineForm}>
          <label>版本<input value={version} onChange={(event) => setVersion(event.target.value)} /></label>
          <label className={styles.grow}>Git 提交<input value={commit} onChange={(event) => setCommit(event.target.value)} placeholder="7 位以上提交标识" /></label>
          <button
            type="button"
            disabled={!canEdit || busy || commit.trim().length < 7}
            onClick={() => run(
              () => erpBlueprintApi.publishVersion(projectId, blueprint.id, manifest()),
              `版本 ${version} 已发布。`,
            )}
          ><PackagePlus size={15} />发布</button>
        </div>
        <div className={styles.rowList}>
          {overview.versions.map((item) => (
            <div key={item.id} className={styles.row}>
              <strong>{item.manifest.version}</strong>
              <span>{item.manifest.source_git_commit.slice(0, 10)}</span>
              <span>{item.manifest.modules.length} 个模块</span>
              <time>{shortDate(item.created_at)}</time>
            </div>
          ))}
          {!overview.versions.length && <p className={styles.empty}>尚未发布版本。</p>}
        </div>
      </section>

      <BlueprintEvolutionForm
        projectId={projectId}
        canEdit={canEdit}
        blueprint={blueprint}
        unreleasedCapabilityKeys={overview.unreleased_capability_keys}
        refresh={refresh}
      />

      <section className={styles.band}>
        <header><Plus size={17} /><h3>创建或纳入商户项目</h3></header>
        <div className={styles.formGrid}>
          <div className={styles.wideField}>
            <div className={styles.segmented} aria-label="商户项目纳入方式">
              <button type="button" data-active={onboardingMode === 'new_project'} onClick={() => selectOnboardingMode('new_project')}>新建项目</button>
              <button type="button" data-active={onboardingMode === 'existing_project'} onClick={() => selectOnboardingMode('existing_project')}>纳入现有项目</button>
            </div>
          </div>
          {onboardingMode === 'new_project' ? (
            <label>项目名称<input value={instanceName} onChange={(event) => setInstanceName(event.target.value)} /></label>
          ) : (
            <label>现有项目
              <select value={targetProjectId} onChange={(event) => setTargetProjectId(event.target.value)}>
                <option value="">请选择可编辑项目</option>
                {eligibleTargetProjects.map((project) => (
                  <option key={project.id} value={project.id}>{project.display_name || project.name}</option>
                ))}
              </select>
            </label>
          )}
          <label>实例标识<input value={instanceKey} onChange={(event) => setInstanceKey(event.target.value)} /></label>
          <label>行业<input value={industry} onChange={(event) => setIndustry(event.target.value)} /></label>
          <label>主题<select value={theme} onChange={(event) => setTheme(event.target.value)}>{blueprint.definition.themes.map((item) => <option key={item}>{item}</option>)}</select></label>
          <label>蓝图版本<select value={targetVersion} onChange={(event) => setTargetVersion(event.target.value)}>{versionOptions.map((item) => <option key={item}>{item}</option>)}</select></label>
          <div className={styles.formAction}>
            <button
              type="button"
              disabled={
                !canEdit
                || busy
                || !targetVersion
                || !instanceKey.trim()
                || (onboardingMode === 'new_project' ? !instanceName.trim() : !targetProjectId)
              }
              onClick={() => run(
                () => erpBlueprintApi.createInstance(projectId, blueprint.id, {
                  instance_key: instanceKey,
                  project_name: onboardingMode === 'new_project' ? instanceName : '',
                  target_project_id: onboardingMode === 'existing_project' ? targetProjectId : undefined,
                  version: targetVersion,
                  industry,
                  theme_key: theme,
                  enabled_modules: [],
                  plugins: [],
                  private_extensions: [],
                }),
                onboardingMode === 'existing_project' ? '现有项目已纳入 ERP。' : '独立商户项目已创建。',
              )}
            >{onboardingMode === 'existing_project' ? <FolderInput size={15} /> : <Plus size={15} />}{onboardingMode === 'existing_project' ? '纳入' : '创建'}</button>
          </div>
        </div>
        <div className={styles.rowList}>
          {overview.instances.map((instance) => (
            <div key={instance.id} className={styles.row}>
              <strong>{instance.instance_key}</strong>
              <span>{instance.industry}</span>
              <span>{instance.theme_key} · {instance.onboarding_mode === 'existing_project' ? '已有项目' : '新建项目'}</span>
              <span>v{instance.pinned_version}</span>
            </div>
          ))}
        </div>
      </section>

      <section className={styles.band}>
        <header><Check size={17} /><h3>通用功能提案</h3></header>
        <div className={styles.proposalList}>
          {overview.proposals.map((proposal) => (
            <article key={proposal.id} className={styles.proposal}>
              <div>
                <strong>{proposal.title}</strong>
                <p>{proposal.summary}</p>
                <small>{proposal.support_count} 个独立商户 · {proposal.industries.join('、') || '行业未标注'} · {proposal.status}</small>
              </div>
              {proposal.status === 'candidate' && (
                <div className={styles.iconActions}>
                  <button
                    type="button"
                    title="接受并创建 Matter"
                    disabled={!canEdit || busy || proposal.support_count < blueprint.definition.proposal_threshold}
                    onClick={() => run(
                      () => erpBlueprintApi.decideProposal(projectId, proposal.id, { decision: 'accepted', note: '维护者确认进入正式开发流程', create_matter: true }),
                      '提案已接受并创建 Matter。',
                    )}
                  ><Check size={15} /></button>
                  <button
                    type="button"
                    title="拒绝提案"
                    disabled={!canEdit || busy}
                    onClick={() => run(
                      () => erpBlueprintApi.decideProposal(projectId, proposal.id, { decision: 'rejected', note: '维护者拒绝', create_matter: false }),
                      '提案已拒绝。',
                    )}
                  ><X size={15} /></button>
                </div>
              )}
              {proposal.status === 'accepted' && (
                <button type="button" disabled={!canEdit || busy} onClick={() => run(
                  () => erpBlueprintApi.createProposalMatter(projectId, proposal.id),
                  'Matter 已创建。',
                )}><RotateCcw size={15} />创建 Matter</button>
              )}
            </article>
          ))}
          {!overview.proposals.length && <p className={styles.empty}>尚无脱敏通用需求信号。</p>}
        </div>
      </section>
      {message && <p className={styles.message}>{message}</p>}
    </div>
  )
}
function Metric({ label, value }: { label: string; value: string }) {
  return <div><span>{label}</span><strong>{value}</strong></div>
}

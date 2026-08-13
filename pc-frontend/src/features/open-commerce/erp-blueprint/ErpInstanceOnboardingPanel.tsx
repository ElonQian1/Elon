import { useCallback, useEffect, useMemo, useState } from 'react'
import { ArrowRight, FolderInput, Plus } from 'lucide-react'
import { erpBlueprintApi } from './erpBlueprintApi'
import ErpExistingProjectRegistrar from './ErpExistingProjectRegistrar'
import type { ErpOverview, ErpTargetProject } from './erpBlueprintTypes'
import { errorMessage } from './erpBlueprintUi'
import styles from './ErpBlueprintPanel.module.css'

type OnboardingMode = 'new_project' | 'existing_project'

export default function ErpInstanceOnboardingPanel({
  projectId,
  canEdit,
  overview,
  refresh,
  onOpenProject,
}: {
  projectId: string
  canEdit: boolean
  overview: ErpOverview
  refresh: () => Promise<void>
  onOpenProject: (projectId: string) => Promise<void>
}) {
  const blueprint = overview.blueprint!
  const [instanceName, setInstanceName] = useState('')
  const [instanceKey, setInstanceKey] = useState('')
  const [onboardingMode, setOnboardingMode] = useState<OnboardingMode>('new_project')
  const [targetProjectId, setTargetProjectId] = useState('')
  const [targetProjects, setTargetProjects] = useState<ErpTargetProject[]>([])
  const [industry, setIndustry] = useState('local_retail')
  const [theme, setTheme] = useState(blueprint.definition.themes[0] ?? 'default.clean')
  const [targetVersion, setTargetVersion] = useState(overview.versions[0]?.manifest.version ?? '')
  const [busy, setBusy] = useState(false)
  const [message, setMessage] = useState('')
  const versionOptions = useMemo(
    () => overview.versions.map((item) => item.manifest.version),
    [overview.versions],
  )
  const eligibleTargetProjects = useMemo(
    () => eligibleProjects(targetProjects, overview, projectId),
    [overview, projectId, targetProjects],
  )

  const loadTargetProjects = useCallback(async (preferredProjectId?: string) => {
    const response = await erpBlueprintApi.listTargetProjects()
    const projects = response.projects ?? []
    const eligible = eligibleProjects(projects, overview, projectId)
    setTargetProjects(projects)
    setTargetProjectId((current) => {
      const requested = preferredProjectId || current
      return eligible.some((project) => project.id === requested)
        ? requested
        : eligible[0]?.id ?? ''
    })
    if (preferredProjectId && !eligible.some((project) => project.id === preferredProjectId)) {
      throw new Error('项目已登记，但当前账号没有编辑权，或该项目已经纳入 ERP。')
    }
  }, [overview, projectId])

  useEffect(() => {
    void loadTargetProjects().catch(() => setTargetProjects([]))
  }, [loadTargetProjects])

  function selectOnboardingMode(mode: OnboardingMode) {
    setOnboardingMode(mode)
    if (mode === 'existing_project' && !targetProjectId) {
      setTargetProjectId(eligibleTargetProjects[0]?.id ?? '')
    }
  }

  async function createInstance() {
    setBusy(true)
    setMessage('')
    let createdProjectId = ''
    try {
      const instance = await erpBlueprintApi.createInstance(projectId, blueprint.id, {
        instance_key: instanceKey,
        project_name: onboardingMode === 'new_project' ? instanceName : '',
        target_project_id: onboardingMode === 'existing_project' ? targetProjectId : undefined,
        version: targetVersion,
        industry,
        theme_key: theme,
        enabled_modules: [],
        plugins: [],
        private_extensions: [],
      })
      createdProjectId = instance.project_id
      await onOpenProject(instance.project_id)
    } catch (error) {
      if (createdProjectId) {
        await refresh().catch(() => {})
        setMessage(`项目已纳入 ERP，但未能自动打开商户项目：${errorMessage(error)}`)
      } else {
        setMessage(errorMessage(error))
      }
    } finally {
      setBusy(false)
    }
  }

  async function openInstanceProject(instanceProjectId: string) {
    setBusy(true)
    setMessage('')
    try {
      await onOpenProject(instanceProjectId)
    } catch (error) {
      setMessage(errorMessage(error))
      setBusy(false)
    }
  }

  return (
    <section className={styles.band}>
      <header><Plus size={17} /><h3>创建或纳入商户项目</h3></header>
      <p className={styles.mutedLine}>登记本机目录只建立平台项目；点击“纳入”后才绑定 ERP。成功后进入商户项目，由商户确认配置并创建初始化 Matter。</p>
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
          <>
            <label>现有项目
              <select value={targetProjectId} onChange={(event) => setTargetProjectId(event.target.value)}>
                <option value="">请选择可编辑项目</option>
                {eligibleTargetProjects.map((project) => (
                  <option key={project.id} value={project.id}>{project.display_name || project.name}</option>
                ))}
              </select>
            </label>
            <ErpExistingProjectRegistrar canEdit={canEdit} disabled={busy} onRegistered={loadTargetProjects} />
          </>
        )}
        <label>实例标识<input value={instanceKey} onChange={(event) => setInstanceKey(event.target.value)} /></label>
        <label>行业<input value={industry} onChange={(event) => setIndustry(event.target.value)} /></label>
        <label>主题<select value={theme} onChange={(event) => setTheme(event.target.value)}>{blueprint.definition.themes.map((item) => <option key={item}>{item}</option>)}</select></label>
        <label>蓝图版本<select value={targetVersion} onChange={(event) => setTargetVersion(event.target.value)}>{versionOptions.map((item) => <option key={item}>{item}</option>)}</select></label>
        <div className={styles.formAction}>
          <button
            type="button"
            disabled={!canEdit || busy || !targetVersion || !instanceKey.trim() || (onboardingMode === 'new_project' ? !instanceName.trim() : !targetProjectId)}
            onClick={createInstance}
          >{onboardingMode === 'existing_project' ? <FolderInput size={15} /> : <Plus size={15} />}{onboardingMode === 'existing_project' ? '纳入并继续' : '创建并继续'}</button>
        </div>
      </div>
      <div className={styles.rowList}>
        {overview.instances.map((instance) => (
          <div key={instance.id} className={styles.merchantInstanceRow}>
            <strong>{instance.instance_key}</strong>
            <span>{instance.industry}</span>
            <span>{instance.theme_key} · {instance.onboarding_mode === 'existing_project' ? '已有项目' : '新建项目'}</span>
            <span>v{instance.pinned_version}</span>
            <button type="button" disabled={busy} onClick={() => void openInstanceProject(instance.project_id)}><ArrowRight size={14} />进入商户项目</button>
          </div>
        ))}
      </div>
      {message && <p className={styles.message}>{message}</p>}
    </section>
  )
}

function eligibleProjects(
  projects: ErpTargetProject[],
  overview: ErpOverview,
  blueprintProjectId: string,
) {
  const boundProjectIds = new Set(overview.instances.map((instance) => instance.project_id))
  return projects.filter((project) => (
    project.id !== blueprintProjectId
    && !boundProjectIds.has(project.id)
    && ['owner', 'admin', 'editor'].includes(
      project.viewer_role ?? project.role ?? project.my_role ?? '',
    )
  ))
}

import { useEffect, useMemo, useState } from 'react'
import { Bot, Plus, Save, Trash2 } from 'lucide-react'
import { erpBlueprintApi } from './erpBlueprintApi'
import type { ErpExtension, ErpOverview } from './erpBlueprintTypes'
import { errorMessage } from './erpBlueprintUi'
import styles from './ErpBlueprintPanel.module.css'

export default function ErpInstanceConfigurationPanel({
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
  const contractDrift = overview.materialization?.matter?.plan_contract_matches === false
  const version = overview.versions.find((item) => item.id === instance.pinned_version_id)
  const availableModules = version?.manifest.modules ?? []
  const themes = overview.blueprint?.definition.themes ?? [instance.theme_key]
  const extensionPoints = version?.manifest.extension_points ?? []
  const [theme, setTheme] = useState(instance.theme_key)
  const [modules, setModules] = useState(instance.enabled_modules)
  const [plugins, setPlugins] = useState(instance.plugins)
  const [privateExtensions, setPrivateExtensions] = useState(instance.private_extensions)
  const [confirmed, setConfirmed] = useState(false)
  const [busy, setBusy] = useState(false)
  const [message, setMessage] = useState('')

  useEffect(() => {
    setTheme(instance.theme_key)
    setModules(instance.enabled_modules)
    setPlugins(instance.plugins)
    setPrivateExtensions(instance.private_extensions)
    setConfirmed(false)
  }, [instance.configuration_revision, instance.enabled_modules, instance.plugins, instance.private_extensions, instance.theme_key])

  const changed = useMemo(
    () => theme !== instance.theme_key
      || JSON.stringify(modules) !== JSON.stringify(instance.enabled_modules)
      || JSON.stringify(plugins) !== JSON.stringify(instance.plugins)
      || JSON.stringify(privateExtensions) !== JSON.stringify(instance.private_extensions),
    [instance, modules, plugins, privateExtensions, theme],
  )

  function toggleModule(moduleKey: string, required: boolean) {
    if (required) return
    setModules((current) => current.includes(moduleKey)
      ? current.filter((item) => item !== moduleKey)
      : [...current, moduleKey].sort())
  }

  async function save() {
    setBusy(true)
    setMessage('')
    try {
      await erpBlueprintApi.updateInstanceConfiguration(projectId, instance.id, {
        expected_revision: instance.configuration_revision,
        merchant_confirmed: confirmed,
        theme_key: theme,
        enabled_modules: modules,
        plugins,
        private_extensions: privateExtensions,
      })
      setMessage('实例配置已保存，私有扩展仍只属于当前商户项目。')
      await refresh()
    } catch (error) {
      setMessage(errorMessage(error))
    } finally {
      setBusy(false)
    }
  }

  async function bootstrap() {
    setBusy(true)
    setMessage('')
    try {
      const result = await erpBlueprintApi.createInstanceBootstrapMatter(projectId, instance.id)
      setMessage(`初始化 Matter 已就绪：${result.matter_id}`)
      await refresh()
    } catch (error) {
      setMessage(errorMessage(error))
    } finally {
      setBusy(false)
    }
  }

  return (
    <section className={styles.band}>
      <header><Save size={17} /><h3>商户实例配置</h3></header>
      <div className={styles.configurationGrid}>
        <label>主题<select value={theme} onChange={(event) => setTheme(event.target.value)}>{themes.map((item) => <option key={item}>{item}</option>)}</select></label>
        <div>
          <span className={styles.fieldLabel}>公共模块</span>
          <div className={styles.moduleChecks}>{availableModules.map((item) => (
            <label key={item.module_key} className={styles.checkLabel}>
              <input type="checkbox" checked={modules.includes(item.module_key)} disabled={item.required} onChange={() => toggleModule(item.module_key, item.required)} />
              {item.module_key}{item.required ? '（必需）' : ''}
            </label>
          ))}</div>
        </div>
      </div>
      <ExtensionEditor title="行业插件" values={plugins} modules={modules} extensionPoints={extensionPoints} onChange={setPlugins} />
      <ExtensionEditor title="私有扩展" values={privateExtensions} modules={modules} extensionPoints={extensionPoints} onChange={setPrivateExtensions} />
      <div className={styles.actionRow}>
        <label className={styles.checkLabel}>
          <input type="checkbox" checked={confirmed} onChange={(event) => setConfirmed(event.target.checked)} />
          商户确认本次配置变更
        </label>
        <button type="button" disabled={!canEdit || busy || !changed || !confirmed} onClick={save}><Save size={15} />保存配置</button>
        <button type="button" disabled={!canEdit || busy} onClick={bootstrap}><Bot size={15} />{contractDrift ? '重新规划初始化 Matter' : instance.bootstrap_matter_id ? '确认初始化 Matter' : '创建初始化 Matter'}</button>
      </div>
      <p className={styles.mutedLine}>配置修订 {instance.configuration_revision}{instance.bootstrap_matter_id ? ` · Matter ${instance.bootstrap_matter_id}` : ''}</p>
      {message && <p className={styles.message}>{message}</p>}
    </section>
  )
}

function ExtensionEditor({
  title,
  values,
  modules,
  extensionPoints,
  onChange,
}: {
  title: string
  values: ErpExtension[]
  modules: string[]
  extensionPoints: string[]
  onChange: (next: ErpExtension[]) => void
}) {
  const [key, setKey] = useState('')
  const [point, setPoint] = useState(extensionPoints[0] ?? '')
  const [requiredModule, setRequiredModule] = useState(modules[0] ?? '')

  useEffect(() => {
    if (!point || !extensionPoints.includes(point)) {
      setPoint(extensionPoints[0] ?? '')
    }
  }, [extensionPoints, point])

  useEffect(() => {
    if (!requiredModule || !modules.includes(requiredModule)) {
      setRequiredModule(modules[0] ?? '')
    }
  }, [modules, requiredModule])

  function add() {
    if (!key.trim() || !point) return
    onChange([...values, {
      extension_key: key.trim(),
      version: '1.0.0',
      extension_point: point,
      requires_modules: requiredModule ? [requiredModule] : [],
    }])
    setKey('')
  }

  return (
    <div className={styles.extensionEditor}>
      <strong>{title}</strong>
      <div className={styles.extensionRows}>{values.map((item) => (
        <div key={item.extension_key}>
          <span>{item.extension_key} · {item.extension_point}</span>
          <button type="button" title={`移除 ${item.extension_key}`} onClick={() => onChange(values.filter((value) => value.extension_key !== item.extension_key))}><Trash2 size={14} /></button>
        </div>
      ))}</div>
      <div className={styles.inlineForm}>
        <label className={styles.grow}>扩展标识<input value={key} onChange={(event) => setKey(event.target.value)} /></label>
        <label>扩展点<select value={point} onChange={(event) => setPoint(event.target.value)}>{extensionPoints.map((item) => <option key={item}>{item}</option>)}</select></label>
        <label>依赖模块<select value={requiredModule} onChange={(event) => setRequiredModule(event.target.value)}><option value="">无</option>{modules.map((item) => <option key={item}>{item}</option>)}</select></label>
        <button type="button" disabled={!key.trim() || !point} onClick={add}><Plus size={15} />添加</button>
      </div>
    </div>
  )
}

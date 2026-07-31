import { useState } from 'react'
import { GitPullRequestArrow, Plus } from 'lucide-react'
import { erpBlueprintApi } from './erpBlueprintApi'
import type { ErpBlueprint } from './erpBlueprintTypes'
import { errorMessage } from './erpBlueprintUi'
import styles from './ErpBlueprintPanel.module.css'

type AdditionKind = 'capability' | 'module' | 'theme' | 'extension_point'

export default function BlueprintEvolutionForm({
  projectId,
  canEdit,
  blueprint,
  unreleasedCapabilityKeys,
  refresh,
}: {
  projectId: string
  canEdit: boolean
  blueprint: ErpBlueprint
  unreleasedCapabilityKeys: string[]
  refresh: () => Promise<void>
}) {
  const [kind, setKind] = useState<AdditionKind>('capability')
  const [key, setKey] = useState('')
  const [displayName, setDisplayName] = useState('')
  const [moduleKey, setModuleKey] = useState(blueprint.definition.modules[0]?.module_key ?? '')
  const [required, setRequired] = useState(false)
  const [busy, setBusy] = useState(false)
  const [message, setMessage] = useState('')

  async function submit() {
    setBusy(true)
    setMessage('')
    try {
      const request = {
        expected_revision: blueprint.definition_revision,
        add_modules: kind === 'module' ? [{
          module_key: key,
          version: '1.0.0',
          kind: 'core',
          required,
          dependencies: [],
        }] : [],
        add_capabilities: kind === 'capability' ? [{
          capability_key: key,
          display_name: displayName,
          description: displayName,
          category: moduleKey,
          module_key: moduleKey,
          aliases: [],
          composable_with: [],
        }] : [],
        add_themes: kind === 'theme' ? [key] : [],
        add_extension_points: kind === 'extension_point' ? [key] : [],
      }
      await erpBlueprintApi.evolveBlueprint(projectId, blueprint.id, request)
      setMessage('蓝图定义已追加，尚未进入商户实例；发布新版本后才会进入版本能力目录。')
      setKey('')
      setDisplayName('')
      await refresh()
    } catch (error) {
      setMessage(errorMessage(error))
    } finally {
      setBusy(false)
    }
  }

  return (
    <section className={styles.band}>
      <header><GitPullRequestArrow size={17} /><h3>追加蓝图定义</h3></header>
      <div className={styles.segmented}>
        {([
          ['capability', '能力'],
          ['module', '模块'],
          ['theme', '主题'],
          ['extension_point', '扩展点'],
        ] as Array<[AdditionKind, string]>).map(([value, label]) => (
          <button key={value} type="button" data-active={kind === value} onClick={() => setKind(value)}>{label}</button>
        ))}
      </div>
      <div className={styles.inlineForm}>
        <label className={styles.grow}>
          {kind === 'capability' ? '能力标识' : kind === 'module' ? '模块标识' : kind === 'theme' ? '主题标识' : '扩展点标识'}
          <input value={key} onChange={(event) => setKey(event.target.value)} placeholder="小写字母、数字和点" />
        </label>
        {kind === 'capability' && (
          <>
            <label className={styles.grow}>显示名称<input value={displayName} onChange={(event) => setDisplayName(event.target.value)} /></label>
            <label>所属模块<select value={moduleKey} onChange={(event) => setModuleKey(event.target.value)}>{blueprint.definition.modules.map((item) => <option key={item.module_key}>{item.module_key}</option>)}</select></label>
          </>
        )}
        {kind === 'module' && (
          <label className={styles.checkLabel}>
            <input type="checkbox" checked={required} onChange={(event) => setRequired(event.target.checked)} />
            新实例必需模块
          </label>
        )}
        <button type="button" disabled={!canEdit || busy || !key.trim() || (kind === 'capability' && !displayName.trim())} onClick={submit}>
          <Plus size={15} />追加
        </button>
      </div>
      {!!blueprint.definition_revision && <p className={styles.mutedLine}>当前定义修订 {blueprint.definition_revision}</p>}
      {!!unreleasedCapabilityKeys.length && (
        <div className={styles.chips} aria-label="尚未发布的能力">
          {unreleasedCapabilityKeys.map((item) => <span key={item}>待发布 · {item}</span>)}
        </div>
      )}
      {message && <p className={styles.message}>{message}</p>}
    </section>
  )
}

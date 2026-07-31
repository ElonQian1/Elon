import { useState } from 'react'
import { Boxes, ShieldCheck } from 'lucide-react'
import { starterCapabilities, starterExtensionPoints, starterModules, starterThemes } from './blueprintDefaults'
import { erpBlueprintApi } from './erpBlueprintApi'
import { errorMessage } from './erpBlueprintUi'
import styles from './ErpBlueprintPanel.module.css'

export default function BlueprintSetupForm({
  projectId,
  canEdit,
  onCreated,
}: {
  projectId: string
  canEdit: boolean
  onCreated: () => Promise<void>
}) {
  const [key, setKey] = useState('official.erp')
  const [name, setName] = useState('一龙通用商户 ERP')
  const [description, setDescription] = useState('供小商户独立部署、定制主题和扩展能力的官方 ERP 蓝图。')
  const [threshold, setThreshold] = useState(3)
  const [busy, setBusy] = useState(false)
  const [message, setMessage] = useState('')

  async function submit(event: React.FormEvent) {
    event.preventDefault()
    setBusy(true)
    setMessage('')
    try {
      await erpBlueprintApi.createBlueprint(projectId, {
        blueprint_key: key,
        name,
        description,
        modules: starterModules,
        capabilities: starterCapabilities,
        themes: starterThemes,
        extension_points: starterExtensionPoints,
        proposal_threshold: threshold,
      })
      setMessage('蓝图已登记。下一步发布第一个不可变版本。')
      await onCreated()
    } catch (error) {
      setMessage(errorMessage(error))
    } finally {
      setBusy(false)
    }
  }

  return (
    <section className={styles.setup}>
      <div className={styles.setupIntro}>
        <Boxes size={24} aria-hidden="true" />
        <div>
          <h3>把当前项目登记为 ERP 蓝图</h3>
          <p>默认包含商品、订单、库存、会员、财务和营销模块。商户实例仍会创建为独立项目。</p>
        </div>
      </div>
      <form className={styles.formGrid} onSubmit={submit}>
        <label>
          蓝图标识
          <input value={key} onChange={(event) => setKey(event.target.value)} disabled={!canEdit} />
        </label>
        <label>
          蓝图名称
          <input value={name} onChange={(event) => setName(event.target.value)} disabled={!canEdit} />
        </label>
        <label className={styles.wideField}>
          用途
          <textarea value={description} onChange={(event) => setDescription(event.target.value)} disabled={!canEdit} />
        </label>
        <label>
          独立商户支持阈值
          <input
            type="number"
            min={2}
            max={100}
            value={threshold}
            onChange={(event) => setThreshold(Number(event.target.value))}
            disabled={!canEdit}
          />
        </label>
        <div className={styles.formAction}>
          <button type="submit" disabled={!canEdit || busy}>
            <ShieldCheck size={15} aria-hidden="true" />
            {busy ? '登记中…' : '登记官方蓝图'}
          </button>
        </div>
      </form>
      {message && <p className={styles.message}>{message}</p>}
    </section>
  )
}

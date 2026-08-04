import { useMemo, useState, type FormEvent } from 'react'
import { Cpu, LoaderCircle, X } from 'lucide-react'
import {
  type ComputeDataClass,
  type CreateMyComputeProviderBody,
  type MyComputeProviderKind,
} from './myComputeSettlementApi'
import styles from './CreateComputeProviderDialog.module.css'

interface CreateComputeProviderDialogProps {
  busy: boolean
  error: string
  onClose: () => void
  onSubmit: (body: CreateMyComputeProviderBody) => Promise<void>
}

const DATA_CLASSES: Array<{ value: ComputeDataClass; label: string }> = [
  { value: 'public', label: '公开数据' },
  { value: 'low_sensitivity', label: '低敏感数据' },
  { value: 'restricted', label: '受限数据' },
]

export default function CreateComputeProviderDialog({
  busy,
  error,
  onClose,
  onSubmit,
}: CreateComputeProviderDialogProps) {
  const [providerId] = useState(createProviderId)
  const [kind, setKind] = useState<MyComputeProviderKind>('user_node')
  const [displayName, setDisplayName] = useState('')
  const [homeRegion, setHomeRegion] = useState('')
  const [taskKinds, setTaskKinds] = useState('llm_inference')
  const [acceleratorKinds, setAcceleratorKinds] = useState('gpu')
  const [regions, setRegions] = useState('')
  const [dataClasses, setDataClasses] = useState<ComputeDataClass[]>(['public'])
  const [supportsStreaming, setSupportsStreaming] = useState(true)
  const [supportsCheckpointing, setSupportsCheckpointing] = useState(false)
  const [hardwareDigest, setHardwareDigest] = useState('')
  const parsedTaskKinds = useMemo(() => parseList(taskKinds), [taskKinds])
  const parsedAccelerators = useMemo(() => parseList(acceleratorKinds), [acceleratorKinds])
  const parsedRegions = useMemo(() => parseList(regions), [regions])
  const canSubmit = Boolean(
    displayName.trim()
    && parsedTaskKinds.length
    && parsedAccelerators.length
    && !busy,
  )

  function toggleDataClass(value: ComputeDataClass) {
    setDataClasses((current) => current.includes(value)
      ? current.filter((item) => item !== value)
      : [...current, value])
  }

  async function submit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault()
    if (!canSubmit) return
    await onSubmit({
      provider_id: providerId,
      provider_kind: kind,
      display_name: displayName.trim(),
      home_region: homeRegion.trim() || null,
      task_kinds: parsedTaskKinds,
      accelerator_kinds: parsedAccelerators,
      regions: parsedRegions,
      allowed_data_classes: dataClasses,
      supports_streaming: supportsStreaming,
      supports_checkpointing: supportsCheckpointing,
      declared_hardware_digest: hardwareDigest.trim() || null,
    })
  }

  return (
    <div className={styles.backdrop} onMouseDown={(event) => event.target === event.currentTarget && !busy && onClose()}>
      <section className={styles.dialog} role="dialog" aria-modal="true" aria-labelledby="create-provider-title">
        <header className={styles.header}>
          <div>
            <span>自我声明供给</span>
            <h2 id="create-provider-title">登记算力 Provider</h2>
          </div>
          <button type="button" className={styles.iconButton} onClick={onClose} disabled={busy} aria-label="关闭" title="关闭"><X size={18} /></button>
        </header>

        <div className={styles.identity}>
          <Cpu size={18} aria-hidden="true" />
          <div><span>Provider ID</span><strong>{providerId}</strong></div>
        </div>

        <form onSubmit={(event) => void submit(event)}>
          {error && <div className={styles.error} role="alert">{error}</div>}
          <div className={styles.segmented} aria-label="Provider 类型">
            <button type="button" data-active={kind === 'user_node'} onClick={() => setKind('user_node')}>个人节点</button>
            <button type="button" data-active={kind === 'managed_cluster'} onClick={() => setKind('managed_cluster')}>托管集群</button>
          </div>

          <div className={styles.twoColumns}>
            <label className={styles.field}>
              <span>显示名称</span>
              <input value={displayName} onChange={(event) => setDisplayName(event.target.value)} maxLength={160} placeholder="例如：店内收银机 GPU" autoFocus required />
            </label>
            <label className={styles.field}>
              <span>所属区域</span>
              <input value={homeRegion} onChange={(event) => setHomeRegion(event.target.value)} maxLength={80} placeholder="例如：cn-jiangxi-jian" />
            </label>
          </div>

          <label className={styles.field}>
            <span>任务类型</span>
            <input value={taskKinds} onChange={(event) => setTaskKinds(event.target.value)} placeholder="逗号分隔，例如 llm_inference, image_generation" required />
          </label>
          <label className={styles.field}>
            <span>加速器类型</span>
            <input value={acceleratorKinds} onChange={(event) => setAcceleratorKinds(event.target.value)} placeholder="逗号分隔，例如 gpu, cpu" required />
          </label>
          <label className={styles.field}>
            <span>服务区域</span>
            <input value={regions} onChange={(event) => setRegions(event.target.value)} placeholder="可选，逗号分隔" />
          </label>

          <fieldset className={styles.dataClasses}>
            <legend>允许处理的数据分类</legend>
            {DATA_CLASSES.map((item) => (
              <label key={item.value}><input type="checkbox" checked={dataClasses.includes(item.value)} onChange={() => toggleDataClass(item.value)} /><span>{item.label}</span></label>
            ))}
          </fieldset>

          <div className={styles.capabilities}>
            <label><input type="checkbox" checked={supportsStreaming} onChange={(event) => setSupportsStreaming(event.target.checked)} /><span>支持流式结果</span></label>
            <label><input type="checkbox" checked={supportsCheckpointing} onChange={(event) => setSupportsCheckpointing(event.target.checked)} /><span>支持检查点恢复</span></label>
          </div>

          <label className={styles.field}>
            <span>声明硬件摘要</span>
            <input value={hardwareDigest} onChange={(event) => setHardwareDigest(event.target.value)} maxLength={256} placeholder="可选；仅作为声明，不代表平台验证" />
          </label>

          <div className={styles.boundary}>新 Provider 固定为“登记中 / 自我声明”，不会自动绑定节点、激活供给或发布报价。</div>

          <footer className={styles.footer}>
            <button type="button" className={styles.cancelButton} onClick={onClose} disabled={busy}>取消</button>
            <button type="submit" className={styles.submitButton} disabled={!canSubmit}>
              {busy && <LoaderCircle size={15} className={styles.spinning} aria-hidden="true" />}
              {busy ? '正在登记' : '确认登记'}
            </button>
          </footer>
        </form>
      </section>
    </div>
  )
}

function parseList(value: string) {
  return [...new Set(value.split(/[,，]/).map((item) => item.trim()).filter(Boolean))]
}

function createProviderId() {
  return `provider-${globalThis.crypto.randomUUID()}`
}

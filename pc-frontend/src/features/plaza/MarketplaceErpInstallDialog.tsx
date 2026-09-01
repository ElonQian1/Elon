import { useEffect, useState } from 'react'
import { ArrowRight, Loader2, Store, X } from 'lucide-react'
import { api } from '../../api/client'
import type { PlazaProject } from './ProjectPlazaView'
import styles from './MarketplaceErpInstallDialog.module.css'

interface MarketplaceInstanceResult {
  instance: {
    project_id: string
  }
  target_route: string
}

interface Props {
  project: PlazaProject
  onClose: () => void
  onCreated: (result: MarketplaceInstanceResult) => Promise<void>
}

export default function MarketplaceErpInstallDialog({ project, onClose, onCreated }: Props) {
  const [projectName, setProjectName] = useState('我的店铺')
  const [industry, setIndustry] = useState('local_retail')
  const [busy, setBusy] = useState(false)
  const [error, setError] = useState('')

  useEffect(() => {
    function handleKeyDown(event: KeyboardEvent) {
      if (event.key === 'Escape' && !busy) onClose()
    }
    window.addEventListener('keydown', handleKeyDown)
    return () => window.removeEventListener('keydown', handleKeyDown)
  }, [busy, onClose])

  async function handleSubmit(event: React.FormEvent) {
    event.preventDefault()
    const normalizedName = projectName.trim()
    if (!normalizedName) {
      setError('请填写店铺名称')
      return
    }
    setBusy(true)
    setError('')
    try {
      const result = await api.post<MarketplaceInstanceResult>(
        `/api/store/projects/${encodeURIComponent(project.id)}/erp-instances`,
        { project_name: normalizedName, industry },
      )
      await onCreated(result)
    } catch (caught) {
      setError((caught as { message?: string }).message ?? '店铺创建失败')
      setBusy(false)
    }
  }

  return (
    <div className={styles.backdrop} role="presentation" onMouseDown={() => !busy && onClose()}>
      <section
        className={styles.dialog}
        role="dialog"
        aria-modal="true"
        aria-labelledby="marketplace-install-title"
        onMouseDown={(event) => event.stopPropagation()}
      >
        <header className={styles.header}>
          <span className={styles.icon}><Store size={19} aria-hidden="true" /></span>
          <div>
            <strong id="marketplace-install-title">创建我的店铺</strong>
            <span>基于 {project.display_name || project.name}</span>
          </div>
          <button
            className={styles.close}
            type="button"
            title="关闭"
            aria-label="关闭"
            disabled={busy}
            onClick={onClose}
          >
            <X size={17} aria-hidden="true" />
          </button>
        </header>

        <form className={styles.form} onSubmit={handleSubmit}>
          <label className={styles.field}>
            <span>店铺名称</span>
            <input
              autoFocus
              maxLength={80}
              value={projectName}
              onChange={(event) => setProjectName(event.target.value)}
              placeholder="例如：钱一龙咖啡店"
            />
          </label>
          <label className={styles.field}>
            <span>经营类型</span>
            <select value={industry} onChange={(event) => setIndustry(event.target.value)}>
              <option value="local_retail">本地零售</option>
              <option value="coffee">咖啡与饮品</option>
              <option value="restaurant">餐饮外卖</option>
              <option value="convenience">便利店</option>
            </select>
          </label>

          <p className={styles.notice}>
            系统会创建一个只属于你的独立项目。平台账号登录和商户经营数据不会写入公开模板项目。
          </p>
          {error && <div className={styles.error} role="alert">{error}</div>}

          <footer className={styles.actions}>
            <button className={styles.cancel} type="button" disabled={busy} onClick={onClose}>取消</button>
            <button className={styles.submit} type="submit" disabled={busy}>
              {busy ? <Loader2 size={15} aria-hidden="true" /> : <ArrowRight size={15} aria-hidden="true" />}
              <span>{busy ? '正在创建…' : '创建并进入'}</span>
            </button>
          </footer>
        </form>
      </section>
    </div>
  )
}

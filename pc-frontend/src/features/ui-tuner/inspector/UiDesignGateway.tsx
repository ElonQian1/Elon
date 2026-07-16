import styles from './UiDesignGateway.module.css'

interface UiDesignGatewayProps {
  title: string
  detail: string
  onCreateDraft: () => void
  onConnectLive: () => void
  onOpenAi: () => void
  liveDisabled?: boolean
}

export function UiDesignGateway({
  title,
  detail,
  onCreateDraft,
  onConnectLive,
  onOpenAi,
  liveDisabled,
}: UiDesignGatewayProps) {
  return (
    <section className={styles.gateway}>
      <span className={styles.eyebrow}>当前为 Android 只读真帧</span>
      <strong>{title}</strong>
      <p>{detail}</p>
      <button className={styles.primary} type="button" onClick={onCreateDraft}>建立可编辑草稿</button>
      <div className={styles.secondary}>
        <button type="button" disabled={liveDisabled} onClick={onConnectLive}>连接 LIVE Runtime</button>
        <button type="button" onClick={onOpenAi}>让 AI 建立绑定</button>
      </div>
      <small>草稿负责即时设计，Android 真帧负责最终校准；写回后仍需无 Patch 构建验证。</small>
    </section>
  )
}

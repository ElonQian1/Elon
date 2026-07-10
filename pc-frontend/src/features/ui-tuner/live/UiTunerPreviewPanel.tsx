import { useState } from 'react'
import type { LivePreviewRequest } from './liveUiApi'
import styles from './UiTunerLivePanel.module.css'

export function UiTunerPreviewPanel({
  busy,
  onOpen,
}: {
  busy: boolean
  onOpen: (request: LivePreviewRequest) => Promise<void>
}) {
  const [request, setRequest] = useState<LivePreviewRequest>({
    screenId: 'elon.compose.gallery',
    scenario: 'normal',
    theme: 'system',
    fontScale: 1,
    locale: 'zh-CN',
  })
  const update = <K extends keyof LivePreviewRequest>(key: K, value: LivePreviewRequest[K]) => {
    setRequest((current) => ({ ...current, [key]: value }))
  }
  return (
    <div className={styles.solver}>
      <div>
        <strong>Preview 场景</strong>
        <span>固定数据、主题、字号与语言</span>
      </div>
      <label className={styles.fieldFull}>
        <span>Screen ID</span>
        <input
          value={request.screenId}
          disabled={busy}
          onChange={(event) => update('screenId', event.currentTarget.value)}
        />
      </label>
      <div className={styles.grid}>
        <label className={styles.field}>
          <span>场景</span>
          <select
            value={request.scenario}
            disabled={busy}
            onChange={(event) => update('scenario', event.currentTarget.value as LivePreviewRequest['scenario'])}
          >
            <option value="normal">normal</option>
            <option value="loading">loading</option>
            <option value="empty">empty</option>
            <option value="error">error</option>
          </select>
        </label>
        <label className={styles.field}>
          <span>主题</span>
          <select
            value={request.theme}
            disabled={busy}
            onChange={(event) => update('theme', event.currentTarget.value as LivePreviewRequest['theme'])}
          >
            <option value="system">system</option>
            <option value="light">light</option>
            <option value="dark">dark</option>
          </select>
        </label>
        <label className={styles.field}>
          <span>字体倍率</span>
          <select
            value={request.fontScale}
            disabled={busy}
            onChange={(event) => update('fontScale', Number(event.currentTarget.value))}
          >
            <option value={1}>1.0</option>
            <option value={1.3}>1.3</option>
            <option value={1.5}>1.5</option>
          </select>
        </label>
        <label className={styles.field}>
          <span>语言</span>
          <select
            value={request.locale}
            disabled={busy}
            onChange={(event) => update('locale', event.currentTarget.value)}
          >
            <option value="zh-CN">zh-CN</option>
            <option value="en-US">en-US</option>
          </select>
        </label>
      </div>
      <button
        className={styles.commitButton}
        type="button"
        disabled={busy || !request.screenId.trim()}
        onClick={() => { void onOpen({ ...request, screenId: request.screenId.trim() }) }}
      >
        打开真实 Preview Host
      </button>
      <small>当前内置 elon.view.gallery 与 elon.compose.gallery，用于验证两种渲染器。</small>
    </div>
  )
}

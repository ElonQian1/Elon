import { useEffect, useRef, useState } from 'react'
import { createPortal } from 'react-dom'
import { useModelStore } from './useModelStore'
import { useAuthStore } from '../../store/auth'
import { providerGroupTitle, shortButtonLabel } from './modelUtils'
import type { AgentOption } from './types'
import styles from './ModelPicker.module.css'

interface Props {
  /** 触发按钮的 ref，用于定位 popover */
  anchorRef: React.RefObject<HTMLElement | null>
  onClose: () => void
}

export function ModelPickerPopover({ anchorRef, onClose }: Props) {
  const user = useAuthStore((s) => s.user)
  const { options, selectedAgent, label, codexCliOnly, loading, error, load, saveSelection } =
    useModelStore()
  const [saving, setSaving] = useState(false)
  const [saveError, setSaveError] = useState('')
  const [pos, setPos] = useState({ left: 12, bottom: 12, width: 360 })

  // 定位 popover（锚点按钮上方）
  useEffect(() => {
    function reposition() {
      const el = anchorRef.current
      if (!el) return
      const rect = el.getBoundingClientRect()
      const width = Math.min(360, window.innerWidth - 24)
      const left = Math.max(12, Math.min(rect.left, window.innerWidth - width - 12))
      const bottom = Math.max(12, window.innerHeight - rect.top + 8)
      setPos({ left: Math.round(left), bottom: Math.round(bottom), width })
    }
    reposition()
    window.addEventListener('resize', reposition)
    return () => window.removeEventListener('resize', reposition)
  }, [anchorRef])

  // 初次打开时加载
  useEffect(() => {
    if (user?.id && !loading) load(user.id)
  // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [user?.id])

  // Escape 关闭
  useEffect(() => {
    const handler = (e: KeyboardEvent) => { if (e.key === 'Escape') onClose() }
    document.addEventListener('keydown', handler)
    return () => document.removeEventListener('keydown', handler)
  }, [onClose])

  async function handleSelect(option: AgentOption) {
    if (!user?.id) return
    setSaving(true)
    setSaveError('')
    try {
      await saveSelection(option, user.id)
      onClose()
    } catch (err) {
      setSaveError((err as { message?: string }).message ?? '切换失败')
    } finally {
      setSaving(false)
    }
  }

  // 按 provider 分组
  const groups = new Map<string, AgentOption[]>()
  for (const opt of options) {
    const title = providerGroupTitle(opt.provider)
    if (!groups.has(title)) groups.set(title, [])
    groups.get(title)!.push(opt)
  }

  const popover = (
    <>
      <button
        className={styles.backdrop}
        type="button"
        aria-label="关闭模型选择"
        onClick={onClose}
      />
      <section
        className={styles.popover}
        role="dialog"
        aria-label="选择 AI 模型"
        style={{ left: pos.left, bottom: pos.bottom, width: pos.width }}
      >
        <header className={styles.header}>
          <div>
            <strong>选择 AI 模型</strong>
            <span>{label || '服务器默认'}</span>
          </div>
          <button className={styles.closeBtn} type="button" onClick={onClose} aria-label="关闭">
            ×
          </button>
        </header>

        <div className={styles.list}>
          {loading && <p className={styles.empty}>正在读取模型列表…</p>}
          {!loading && (error || saveError) && (
            <p className={styles.errorMsg}>{saveError || error}</p>
          )}
          {!loading && !error && options.length === 0 && (
            <p className={styles.empty}>
              当前没有可选模型。请检查服务器 agent 配置或 PC 节点 CLI 配置。
            </p>
          )}
          {!loading &&
            Array.from(groups.entries()).map(([title, opts]) => (
              <div key={title}>
                {title !== '默认' && <div className={styles.section}>{title}</div>}
                {opts.map((opt) => (
                  <button
                    key={opt.agentName || '__default__'}
                    className={[styles.option, opt.agentName === selectedAgent ? styles.active : ''].join(' ')}
                    type="button"
                    disabled={saving}
                    onClick={() => handleSelect(opt)}
                  >
                    <span>
                      <strong>{opt.label}</strong>
                      {opt.subtitle && <span>{opt.subtitle}</span>}
                    </span>
                    <span className={styles.check}>
                      {opt.agentName === selectedAgent ? '✓' : ''}
                    </span>
                  </button>
                ))}
              </div>
            ))}
        </div>

        <footer className={styles.footer}>
          <button type="button" disabled={saving} onClick={() => user?.id && load(user.id)}>
            刷新
          </button>
          {!codexCliOnly && (
            <button type="button" onClick={() => window.open('/web', '_blank')}>
              完整模型设置
            </button>
          )}
        </footer>
      </section>
    </>
  )

  return createPortal(popover, document.body)
}

/** 触发模型选择器的按钮，嵌入侧边栏或工具栏 */
export function ModelPickerButton({ compact }: { compact?: boolean }) {
  const label = useModelStore((s) => s.label)
  const [open, setOpen] = useState(false)
  const btnRef = useRef<HTMLButtonElement>(null)

  const shortLabel = shortButtonLabel(label)

  if (compact) {
    return (
      <>
        <button
          ref={btnRef}
          style={{
            width: 48, height: 48, borderRadius: '50%',
            background: '#1e2026', border: '1px solid #3b3e46',
            color: '#c5c8d0', fontSize: 11, fontWeight: 700,
            cursor: 'pointer', transition: 'background 0.14s',
            display: 'grid', placeItems: 'center', lineHeight: 1.2,
            textAlign: 'center', padding: '2px',
          }}
          title={`AI 模型：${label || '服务器默认'}`}
          onClick={() => setOpen((v) => !v)}
          type="button"
        >
          {shortLabel.length > 5 ? shortLabel.slice(0, 5) : shortLabel}
        </button>
        {open && <ModelPickerPopover anchorRef={btnRef} onClose={() => setOpen(false)} />}
      </>
    )
  }

  return (
    <>
      <button
        ref={btnRef}
        className={styles.triggerBtn}
        type="button"
        title={`AI 模型：${label || '服务器默认'}`}
        onClick={() => setOpen((v) => !v)}
      >
        <span className={styles.triggerIcon}>🧠</span>
        <span className={styles.triggerLabel}>{shortLabel}</span>
      </button>
      {open && (
        <ModelPickerPopover
          anchorRef={btnRef}
          onClose={() => setOpen(false)}
        />
      )}
    </>
  )
}

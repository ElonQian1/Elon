import { useEffect, useState } from 'react'
import type { UiTunerElement } from '../types'
import type {
  LivePatchOperation,
  LiveUiNode,
  LiveUiScope,
  LiveUiSession,
} from './liveUiApi'
import type { LiveUiConnectionState } from './useLiveUiSession'
import styles from './UiTunerLivePanel.module.css'

interface UiTunerLivePanelProps {
  state: LiveUiConnectionState
  error: string
  busy: boolean
  session: LiveUiSession | null
  node: LiveUiNode | null
  onApply: (operation: LivePatchOperation, scope: LiveUiScope) => Promise<unknown>
  onUndo: () => Promise<void>
  onRedo: () => Promise<void>
  onReconnect: () => void
  onOptimisticUpdate: (patch: Partial<UiTunerElement>) => void
}

const NUMBER_FIELDS = [
  ['width', '宽度', 'dp'],
  ['height', '高度', 'dp'],
  ['padding.start', '左内距', 'dp'],
  ['padding.top', '上内距', 'dp'],
  ['padding.end', '右内距', 'dp'],
  ['padding.bottom', '下内距', 'dp'],
  ['cornerRadius.all', '圆角', 'dp'],
  ['textSize', '字号', 'sp'],
  ['borderWidth', '边框', 'dp'],
  ['opacity', '透明度', 'float'],
] as const

const COLOR_FIELDS = [
  ['backgroundColor', '背景色'],
  ['contentColor', '文字色'],
  ['borderColor', '边框色'],
] as const

export function UiTunerLivePanel({
  state,
  error,
  busy,
  session,
  node,
  onApply,
  onUndo,
  onRedo,
  onReconnect,
  onOptimisticUpdate,
}: UiTunerLivePanelProps) {
  const [scope, setScope] = useState<LiveUiScope>('INSTANCE')
  const connected = state === 'connected'

  return (
    <section className={styles.panel} data-state={state}>
      <div className={styles.header}>
        <div>
          <span className={styles.eyebrow}>真机实时样式</span>
          <strong>{statusLabel(state)}</strong>
        </div>
        <span className={styles.statusDot} aria-hidden="true" />
      </div>

      {error && <p className={styles.hint}>{error}</p>}
      {connected && !node && (
        <p className={styles.hint}>当前元素未匹配到 Runtime Node。可继续用右侧 Codex 修改源码，或为 View 添加稳定 uiNode ID。</p>
      )}
      {node && (
        <>
          <div className={styles.nodeInfo}>
            <strong>{node.definitionId}</strong>
            <small>{node.kind} · {node.runtimeNodeId}</small>
          </div>
          <label className={styles.scopeField}>
            <span>作用范围</span>
            <select value={scope} onChange={(event) => setScope(event.currentTarget.value as LiveUiScope)}>
              <option value="INSTANCE">只改当前实例</option>
              <option value="DEFINITION">修改同一组件</option>
            </select>
          </label>

          <div className={styles.grid}>
            {NUMBER_FIELDS.map(([property, label, valueType]) => node.properties[property] && (
              <NumberLiveField
                key={property}
                label={label}
                value={numberValue(node, property)}
                disabled={busy}
                step={property === 'opacity' ? 0.05 : 1}
                onCommit={async (value) => {
                  await onApply({
                    property,
                    value: { type: valueType, value },
                  }, scope)
                  onOptimisticUpdate(optimisticPatch(property, value))
                }}
              />
            ))}
          </div>

          {COLOR_FIELDS.map(([property, label]) => node.properties[property] && (
            <TextLiveField
              key={property}
              label={label}
              value={stringValue(node, property)}
              disabled={busy}
              onCommit={async (value) => {
                await onApply({ property, value: { type: 'argb', value } }, scope)
                onOptimisticUpdate(optimisticPatch(property, value))
              }}
            />
          ))}
          {node.properties.text && (
            <TextLiveField
              label="文案"
              value={stringValue(node, 'text')}
              disabled={busy}
              onCommit={async (value) => {
                await onApply({ property: 'text', value: { type: 'text', value } }, scope)
                onOptimisticUpdate({ text: value })
              }}
            />
          )}

          <div className={styles.actions}>
            <button type="button" disabled={busy || !session?.historyCount} onClick={() => { void onUndo() }}>
              撤销 LIVE
            </button>
            <button type="button" disabled={busy || !session?.redoCount} onClick={() => { void onRedo() }}>
              重做
            </button>
          </div>
          <p className={styles.previewWarning}>LIVE PREVIEW · 真机已变化，源码尚未写入</p>
        </>
      )}

      {!connected && (
        <button className={styles.reconnect} type="button" disabled={state === 'connecting'} onClick={onReconnect}>
          {state === 'connecting' ? '正在自动连接…' : '重新连接 Debug Runtime'}
        </button>
      )}
    </section>
  )
}

function NumberLiveField({
  label,
  value,
  step,
  disabled,
  onCommit,
}: {
  label: string
  value: number
  step: number
  disabled: boolean
  onCommit: (value: number) => Promise<unknown>
}) {
  const [draft, setDraft] = useState(String(value))
  useEffect(() => setDraft(String(value)), [value])
  const commit = () => {
    const next = Number(draft)
    if (!Number.isFinite(next) || next === value) return
    void onCommit(next).catch(() => setDraft(String(value)))
  }
  return (
    <label className={styles.field}>
      <span>{label}</span>
      <input
        type="number"
        value={draft}
        step={step}
        disabled={disabled}
        onChange={(event) => setDraft(event.currentTarget.value)}
        onBlur={commit}
        onKeyDown={(event) => { if (event.key === 'Enter') event.currentTarget.blur() }}
      />
    </label>
  )
}

function TextLiveField({
  label,
  value,
  disabled,
  onCommit,
}: {
  label: string
  value: string
  disabled: boolean
  onCommit: (value: string) => Promise<unknown>
}) {
  const [draft, setDraft] = useState(value)
  useEffect(() => setDraft(value), [value])
  const commit = () => {
    const next = draft.trim()
    if (!next || next === value) return
    void onCommit(next).catch(() => setDraft(value))
  }
  return (
    <label className={styles.fieldFull}>
      <span>{label}</span>
      <input
        value={draft}
        disabled={disabled}
        onChange={(event) => setDraft(event.currentTarget.value)}
        onBlur={commit}
        onKeyDown={(event) => { if (event.key === 'Enter') event.currentTarget.blur() }}
      />
    </label>
  )
}

function numberValue(node: LiveUiNode, property: string) {
  const raw = node.properties[property]?.effective?.value
  return typeof raw === 'number' ? raw : Number(raw) || 0
}

function stringValue(node: LiveUiNode, property: string) {
  return String(node.properties[property]?.effective?.value ?? '')
}

function optimisticPatch(property: string, value: number | string): Partial<UiTunerElement> {
  switch (property) {
    case 'width': return { width: Number(value) }
    case 'height': return { height: Number(value) }
    case 'padding.start':
    case 'padding.end': return { paddingX: Number(value) }
    case 'padding.top':
    case 'padding.bottom': return { paddingY: Number(value) }
    case 'cornerRadius.all': return { borderRadius: Number(value) }
    case 'textSize': return { fontSize: Number(value) }
    case 'borderWidth': return { borderWidth: Number(value) }
    case 'opacity': return { opacity: Number(value) }
    case 'backgroundColor': return { background: normalizeCssColor(String(value)) }
    case 'contentColor': return { color: normalizeCssColor(String(value)) }
    case 'borderColor': return { borderColor: normalizeCssColor(String(value)) }
    default: return {}
  }
}

function normalizeCssColor(value: string) {
  return /^#[0-9a-f]{8}$/i.test(value) ? `#${value.slice(3)}${value.slice(1, 3)}` : value
}

function statusLabel(state: LiveUiConnectionState) {
  switch (state) {
    case 'connected': return '已连接 · LIVE'
    case 'connecting': return '正在连接'
    case 'attach_only': return 'Attach Mode'
    case 'error': return '连接异常'
    default: return '等待真机捕获'
  }
}

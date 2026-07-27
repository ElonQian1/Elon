import { Bot, Copy, Download, MousePointer2, Redo2, RotateCcw, Save, Smartphone, Trash2, Undo2 } from 'lucide-react'
import type { PwaStyleProperty } from './pwaDesignDraft'
import type { PwaDesignSession, PwaSelection } from './usePwaDesignSession'
import { CrossPlatformWritebackReceiptPanel } from './CrossPlatformWritebackReceiptPanel'
import styles from './SourcePreview.module.css'

interface Props {
  session: PwaDesignSession
}

interface FieldSpec {
  property: PwaStyleProperty
  label: string
  placeholder?: string
  quickStep?: number
  quickUnit?: string
  min?: number
  max?: number
}

const SIZE_FIELDS: FieldSpec[] = [
  { property: 'width', label: '宽度', placeholder: 'auto / 100% / 320px', quickStep: 4, quickUnit: 'px', min: 1 },
  { property: 'height', label: '高度', placeholder: 'auto / 100% / 48px', quickStep: 4, quickUnit: 'px', min: 1 },
]

const PADDING_FIELDS: FieldSpec[] = [
  { property: 'paddingTop', label: '上', quickStep: 2, quickUnit: 'px', min: 0 },
  { property: 'paddingRight', label: '右', quickStep: 2, quickUnit: 'px', min: 0 },
  { property: 'paddingBottom', label: '下', quickStep: 2, quickUnit: 'px', min: 0 },
  { property: 'paddingLeft', label: '左', quickStep: 2, quickUnit: 'px', min: 0 },
]

const MARGIN_FIELDS: FieldSpec[] = [
  { property: 'marginTop', label: '上', quickStep: 2, quickUnit: 'px' },
  { property: 'marginRight', label: '右', quickStep: 2, quickUnit: 'px' },
  { property: 'marginBottom', label: '下', quickStep: 2, quickUnit: 'px' },
  { property: 'marginLeft', label: '左', quickStep: 2, quickUnit: 'px' },
]

const TYPE_FIELDS: FieldSpec[] = [
  { property: 'fontSize', label: '字号', placeholder: '16px / 1rem', quickStep: 1, quickUnit: 'px', min: 8 },
  { property: 'fontWeight', label: '字重', placeholder: '400 / 700', quickStep: 100, min: 100, max: 900 },
  { property: 'lineHeight', label: '行高', placeholder: '1.5 / 24px', quickStep: 1, quickUnit: 'px', min: 8 },
  { property: 'borderRadius', label: '圆角', placeholder: '8px / 50%', quickStep: 2, quickUnit: 'px', min: 0 },
]

function originalValue(selection: PwaSelection, property: PwaStyleProperty): string {
  return selection.originalStyle.authored[property]
    || selection.originalStyle.computed[property]
    || ''
}

function fieldValue(session: PwaDesignSession, property: PwaStyleProperty): string {
  const selection = session.selection
  if (!selection) return ''
  return Object.values(session.draft?.elements ?? {}).find((element) => (
    element.identity.key === selection.identity.key || element.identity.selector === selection.identity.selector
  ))?.styleDiff[property]
    ?? originalValue(selection, property)
}

function normalizeNumber(value: number, min?: number, max?: number): number {
  const clampedMin = typeof min === 'number' ? Math.max(min, value) : value
  return typeof max === 'number' ? Math.min(max, clampedMin) : clampedMin
}

function formatNumber(value: number): string {
  return Number.isInteger(value) ? String(value) : value.toFixed(2).replace(/\.?0+$/, '')
}

function adjustedCssValue(current: string, delta: number, fallbackUnit = '', min?: number, max?: number): string {
  const text = current.trim()
  const match = text.match(/^(-?\d+(?:\.\d+)?)([a-z%]*)$/i)
  if (match) {
    const unit = match[2] || fallbackUnit
    return `${formatNumber(normalizeNumber(Number(match[1]) + delta, min, max))}${unit}`
  }
  const fallback = fallbackUnit || 'px'
  return `${formatNumber(normalizeNumber(delta > 0 ? delta : 0, min, max))}${fallback}`
}

function StyleField({ session, spec }: { session: PwaDesignSession; spec: FieldSpec }) {
  const value = fieldValue(session, spec.property)
  const canQuickAdjust = typeof spec.quickStep === 'number'
  const quickAdjust = (direction: -1 | 1) => {
    if (!canQuickAdjust) return
    session.updateStyle(
      spec.property,
      adjustedCssValue(value, spec.quickStep! * direction, spec.quickUnit, spec.min, spec.max),
    )
  }
  return (
    <label className={styles.pwaStyleField}>
      <span>{spec.label}</span>
      <div className={styles.pwaQuickAdjust}>
        {canQuickAdjust && <button type="button" onClick={() => quickAdjust(-1)} aria-label={`${spec.label}减小`}>−</button>}
        <input
          value={value}
          placeholder={spec.placeholder}
          onChange={(event) => session.updateStyle(spec.property, event.currentTarget.value)}
        />
        {canQuickAdjust && <button type="button" onClick={() => quickAdjust(1)} aria-label={`${spec.label}增大`}>+</button>}
      </div>
    </label>
  )
}

function EdgeFields({ session, fields }: { session: PwaDesignSession; fields: FieldSpec[] }) {
  return <div className={styles.pwaEdgeGrid}>{fields.map((spec) => <StyleField key={spec.property} session={session} spec={spec} />)}</div>
}

function confidenceLabel(selection: PwaSelection): string {
  if (selection.identity.confidence === 'high') return '稳定映射'
  if (selection.identity.confidence === 'medium') return '候选映射'
  return 'DOM 路径'
}

function bindingLabel(status?: 'BOUND' | 'CANDIDATE' | 'NEEDS_AI') {
  if (status === 'BOUND') return '双端来源已绑定'
  if (status === 'CANDIDATE') return '已有双端来源候选'
  return '需要 AI 建立绑定'
}

function writebackTargetLabel(status: 'DETERMINISTIC' | 'DETERMINISTIC_PARTIAL' | 'CODEX_REQUIRED') {
  if (status === 'DETERMINISTIC') return '确定性写回'
  if (status === 'DETERMINISTIC_PARTIAL') return '部分确定性 · 部分需要 AI'
  return '需要 AI 建立绑定/结构修改'
}

function WritebackPlanSummary({ session }: Props) {
  const plan = session.writebackPlan
  const deterministicCount = plan.deterministic.pwa.length + plan.deterministic.android.length
  const codexCount = plan.codexChanges.length
  if (!deterministicCount && !codexCount) return null
  return (
    <div className={styles.pwaWritebackPlanSummary}>
      <span>可直接写回 {deterministicCount} 组</span>
      <span>AI 只补 {codexCount} 项</span>
      {plan.codexReasons.slice(0, 2).map((reason) => <small key={reason}>{reason}</small>)}
      {plan.codexReasons.length > 2 && <small>还有 {plan.codexReasons.length - 2} 个绑定缺口会放入 CLI 包。</small>}
    </div>
  )
}

export function PwaStyleInspector({ session }: Props) {
  const selectedDraft = session.selection
    ? Object.values(session.draft?.elements ?? {}).find((element) => (
        element.identity.key === session.selection?.identity.key
        || element.identity.selector === session.selection?.identity.selector
      ))
    : null
  const elementCount = Object.keys(session.draft?.elements ?? {}).length
  return (
    <aside className={styles.pwaStyleInspector} data-testid="pwa-style-inspector">
      <header>
        <div>
          <strong>PWA 手工样式</strong>
          <small>直接修改 iframe 内真实 DOM，页面会立即重排重绘</small>
        </div>
        <span className={styles.pwaDraftCount}>{elementCount} 个元素</span>
      </header>

      <div className={styles.pwaHistoryBar}>
        <button type="button" disabled={!session.canUndo} onClick={session.undo} title="撤销上一事务"><Undo2 size={15} />撤销</button>
        <button type="button" disabled={!session.canRedo} onClick={session.redo} title="重做上一事务"><Redo2 size={15} />重做</button>
        <button type="button" disabled={!session.draft} onClick={session.saveNow}><Save size={15} />保存草稿</button>
      </div>
      <p className={styles.pwaSaveStatus}>{session.saveLabel}</p>

      <section className={styles.pwaSyncCard} data-sync-phase={session.syncState.phase}>
        <strong>{session.syncState.phase}</strong>
        <div className={styles.pwaSyncTargets}>
          <span>PWA：{writebackTargetLabel(session.writebackPlan.targets.pwa)}</span>
          <span>APK：{writebackTargetLabel(session.writebackPlan.targets.android)}</span>
          <strong>{session.writebackPlan.requiresCodex ? '确定性优先，AI 只补缺口' : '无需 AI 重做'}</strong>
        </div>
        <WritebackPlanSummary session={session} />
        <button
          type="button"
          className={styles.pwaPrimarySync}
          data-testid="pwa-cross-platform-sync"
          disabled={!elementCount || session.syncState.phase === 'BUILD_VERIFYING' || Boolean(session.syncState.taskId)}
          onClick={() => { void session.syncNow() }}
        >
          <Bot size={16} />{session.syncState.phase === 'BUILD_VERIFYING'
            ? '正在构建并核验真实源码…'
            : session.syncState.phase === 'AI_WRITING'
              ? 'AI 正在写回源码…'
            : session.writebackPlan.requiresCodex
              ? '让 AI 建立绑定并验证 APK 与 PWA'
              : '写回源码并验证 APK 与 PWA'}
        </button>
        <p className={styles.pwaSyncStatus}>{session.syncState.message}</p>
        <CrossPlatformWritebackReceiptPanel receipt={session.writebackReceipt} />
        {session.syncState.runtimeCapture && <p className={styles.pwaSyncStatus}>
          PNG 证据：{session.syncState.runtimeCapture.width}×{session.syncState.runtimeCapture.height}
          {' · '}<code>{session.syncState.runtimeCapture.sha256.slice(0, 16)}</code>
          {' · '}{session.syncState.runtimeCapture.path}
        </p>}
        {session.syncState.runtimeCaptureDiagnostic && <p className={styles.pwaSyncStatus}>
          {session.syncState.runtimeCaptureDiagnostic.code}：{session.syncState.runtimeCaptureDiagnostic.nextStep}
        </p>}
        {session.syncState.mismatches.length > 0 && <ul>
          {session.syncState.mismatches.map((mismatch) => <li key={mismatch}>{mismatch}</li>)}
        </ul>}
        {session.syncState.phase === 'VERIFY_FAILED' && session.syncState.evidence && (
          <button type="button" onClick={() => { void session.retryVerification() }}>保留草稿并重试真实验证</button>
        )}
        <div className={styles.pwaArtifactActions}>
          <button type="button" disabled={!elementCount} onClick={() => { void session.copyCliPackage() }}><Copy size={13} />复制 CLI 包</button>
          <button type="button" disabled={!elementCount} onClick={session.downloadCliPackage}><Download size={13} />下载草稿</button>
        </div>
      </section>

      {!session.selection && (
        <div className={styles.pwaInspectorEmpty}>
          <strong>{session.mode === 'interact' ? '先正常使用 PWA 到目标页面' : '点击左侧页面中的真实组件'}</strong>
          <span>{session.mode === 'interact' ? '到达目标页面后点击“选择一个组件”，系统只拦截下一次点击；选中后这里会变成样式面板。' : '选中后自动回到正常操作模式，右侧可调尺寸、间距、圆角、字体和颜色。'}</span>
          <button
            type="button"
            className={styles.pwaEmptyPrimaryAction}
            disabled={!session.ready}
            onClick={() => session.setMode(session.mode === 'interact' ? 'select' : 'interact')}
          >
            {session.mode === 'interact'
              ? <><MousePointer2 size={15} />开始选择组件</>
              : <><Smartphone size={15} />返回正常操作</>}
          </button>
          {!session.ready && <small>真实 PWA 连接完成后会自动启用选择。</small>}
        </div>
      )}

      {session.selection && <>
        <section className={styles.pwaSelectedIdentity}>
          <div><strong>{session.selection.identity.ariaLabel || session.selection.identity.text || session.selection.identity.id || session.selection.identity.tag}</strong><span className={`${styles.pwaConfidence} ${styles[`pwaConfidence_${session.selection.identity.confidence}`]}`}>{confidenceLabel(session.selection)}</span></div>
          <code title={session.selection.identity.selector}>{session.selection.identity.selector}</code>
          <div className={styles.pwaBindingSummary}>
            <span>{bindingLabel(selectedDraft?.binding.status)}</span>
            <span>置信度：{selectedDraft?.binding.bindingConfidence ?? session.selection.identity.confidence}</span>
          </div>
          <p>PWA 候选 {selectedDraft?.binding.pwaCandidates.length ?? 0} 个 · Android 候选 {selectedDraft?.binding.androidCandidates.length ?? 0} 个</p>
          {(selectedDraft?.binding.needsBinding ?? session.selection.identity.needsBinding) && <p>需要 AI 建立绑定；selector 只作为本次 Runtime 定位证据，不会成为长期源码身份。</p>}
        </section>

        <section className={styles.pwaStyleSection}>
          <h3>尺寸</h3>
          <div className={styles.pwaTwoColumn}>{SIZE_FIELDS.map((spec) => <StyleField key={spec.property} session={session} spec={spec} />)}</div>
          <small>可直接输入 auto、百分比、rem 或 px，不会强制转成像素。</small>
        </section>

        <section className={styles.pwaStyleSection}>
          <h3>内边距</h3>
          <EdgeFields session={session} fields={PADDING_FIELDS} />
        </section>

        <section className={styles.pwaStyleSection}>
          <h3>外边距</h3>
          <EdgeFields session={session} fields={MARGIN_FIELDS} />
        </section>

        <section className={styles.pwaStyleSection}>
          <h3>形状与文字</h3>
          <div className={styles.pwaTwoColumn}>{TYPE_FIELDS.map((spec) => <StyleField key={spec.property} session={session} spec={spec} />)}</div>
        </section>

        <section className={styles.pwaStyleSection}>
          <h3>颜色与透明度</h3>
          <StyleField session={session} spec={{ property: 'color', label: '文字色', placeholder: '#111827 / rgb(...)' }} />
          <StyleField session={session} spec={{ property: 'backgroundColor', label: '背景色', placeholder: '#ffffff / transparent' }} />
          <label className={styles.pwaOpacityField}>
            <span>透明度</span>
            <input type="range" min="0" max="1" step="0.01" value={Number(fieldValue(session, 'opacity')) || 0} onChange={(event) => session.updateStyle('opacity', event.currentTarget.value)} />
            <input value={fieldValue(session, 'opacity')} onChange={(event) => session.updateStyle('opacity', event.currentTarget.value)} />
          </label>
        </section>

        <div className={styles.pwaResetActions}>
          <button type="button" disabled={!selectedDraft} onClick={session.resetCurrent}><RotateCcw size={15} />重置当前元素</button>
          <button type="button" disabled={!elementCount} onClick={session.clearPage}><Trash2 size={15} />清空本页草稿</button>
        </div>
      </>}
    </aside>
  )
}

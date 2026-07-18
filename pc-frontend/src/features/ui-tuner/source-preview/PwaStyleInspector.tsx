import { Redo2, RotateCcw, Save, Trash2, Undo2 } from 'lucide-react'
import type { PwaStyleProperty } from './pwaDesignDraft'
import type { PwaDesignSession, PwaSelection } from './usePwaDesignSession'
import styles from './SourcePreview.module.css'

interface Props {
  session: PwaDesignSession
}

interface FieldSpec {
  property: PwaStyleProperty
  label: string
  placeholder?: string
}

const SIZE_FIELDS: FieldSpec[] = [
  { property: 'width', label: '宽度', placeholder: 'auto / 100% / 320px' },
  { property: 'height', label: '高度', placeholder: 'auto / 100% / 48px' },
]

const PADDING_FIELDS: FieldSpec[] = [
  { property: 'paddingTop', label: '上' },
  { property: 'paddingRight', label: '右' },
  { property: 'paddingBottom', label: '下' },
  { property: 'paddingLeft', label: '左' },
]

const MARGIN_FIELDS: FieldSpec[] = [
  { property: 'marginTop', label: '上' },
  { property: 'marginRight', label: '右' },
  { property: 'marginBottom', label: '下' },
  { property: 'marginLeft', label: '左' },
]

const TYPE_FIELDS: FieldSpec[] = [
  { property: 'fontSize', label: '字号', placeholder: '16px / 1rem' },
  { property: 'fontWeight', label: '字重', placeholder: '400 / 700' },
  { property: 'lineHeight', label: '行高', placeholder: '1.5 / 24px' },
  { property: 'borderRadius', label: '圆角', placeholder: '8px / 50%' },
]

function originalValue(selection: PwaSelection, property: PwaStyleProperty): string {
  return selection.originalStyle.authored[property]
    || selection.originalStyle.computed[property]
    || ''
}

function fieldValue(session: PwaDesignSession, property: PwaStyleProperty): string {
  const selection = session.selection
  if (!selection) return ''
  return session.draft?.elements[selection.identity.selector]?.styleDiff[property]
    ?? originalValue(selection, property)
}

function StyleField({ session, spec }: { session: PwaDesignSession; spec: FieldSpec }) {
  return (
    <label className={styles.pwaStyleField}>
      <span>{spec.label}</span>
      <input
        value={fieldValue(session, spec.property)}
        placeholder={spec.placeholder}
        onChange={(event) => session.updateStyle(spec.property, event.currentTarget.value)}
      />
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

export function PwaStyleInspector({ session }: Props) {
  const selectedDraft = session.selection
    ? session.draft?.elements[session.selection.identity.selector]
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

      {!session.selection && (
        <div className={styles.pwaInspectorEmpty}>
          <strong>{session.mode === 'interact' ? '先在左侧正常使用 PWA' : '请点击页面中的真实元素'}</strong>
          <span>{session.mode === 'interact' ? '到达目标页面后点击“开始设计/修改页面”。' : '选中后即可调整尺寸、间距、文字和颜色。'}</span>
        </div>
      )}

      {session.selection && <>
        <section className={styles.pwaSelectedIdentity}>
          <div><strong>{session.selection.identity.ariaLabel || session.selection.identity.text || session.selection.identity.id || session.selection.identity.tag}</strong><span className={`${styles.pwaConfidence} ${styles[`pwaConfidence_${session.selection.identity.confidence}`]}`}>{confidenceLabel(session.selection)}</span></div>
          <code title={session.selection.identity.selector}>{session.selection.identity.selector}</code>
          {session.selection.identity.needsBinding && <p>此元素尚未显式绑定源码；草稿保留可解释 selector，并标记为下一阶段待绑定。</p>}
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

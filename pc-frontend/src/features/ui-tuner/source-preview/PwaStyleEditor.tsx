import {
  Box,
  ChevronDown,
  Move,
  Palette,
  Sparkles,
  Type,
  type LucideIcon,
} from 'lucide-react'
import { useState, type ReactNode } from 'react'
import type { PwaStyleProperty } from './pwaDesignDraft'
import { PwaColorStyleField } from './PwaColorStyleField'
import {
  EdgeFields,
  fieldValue,
  MARGIN_FIELDS,
  PADDING_FIELDS,
  SIZE_FIELDS,
  StyleField,
  TYPE_FIELDS,
} from './PwaStyleFields'
import type { PwaDesignSession } from './usePwaDesignSession'
import styles from './PwaStyleInspector.module.css'
import legacyStyles from './SourcePreview.module.css'

interface Props {
  session: PwaDesignSession
}

interface StylePreset {
  label: string
  hint: string
  styles: Partial<Record<PwaStyleProperty, string>>
}

const STYLE_PRESETS: StylePreset[] = [
  {
    label: '紧凑',
    hint: '小按钮与列表项',
    styles: { paddingTop: '6px', paddingRight: '10px', paddingBottom: '6px', paddingLeft: '10px', borderRadius: '10px', fontSize: '13px' },
  },
  {
    label: '标准',
    hint: '常规移动端',
    styles: { paddingTop: '10px', paddingRight: '14px', paddingBottom: '10px', paddingLeft: '14px', borderRadius: '14px', fontSize: '14px' },
  },
  {
    label: '舒展',
    hint: '主操作与卡片',
    styles: { paddingTop: '14px', paddingRight: '18px', paddingBottom: '14px', paddingLeft: '18px', borderRadius: '18px', fontSize: '15px' },
  },
  {
    label: '胶囊',
    hint: '圆润按钮',
    styles: { borderRadius: '999px', paddingLeft: '18px', paddingRight: '18px' },
  },
]

const LAYOUT_PROPERTIES: PwaStyleProperty[] = ['width', 'height']
const SPACING_PROPERTIES: PwaStyleProperty[] = [
  'paddingTop', 'paddingRight', 'paddingBottom', 'paddingLeft',
  'marginTop', 'marginRight', 'marginBottom', 'marginLeft',
]
const TYPE_PROPERTIES: PwaStyleProperty[] = ['fontSize', 'fontWeight', 'lineHeight', 'borderRadius']
const APPEARANCE_PROPERTIES: PwaStyleProperty[] = ['color', 'backgroundColor', 'opacity']

function currentDraftStyles(session: PwaDesignSession): Partial<Record<PwaStyleProperty, string>> {
  if (!session.selection) return {}
  return Object.values(session.draft?.elements ?? {}).find((element) => (
    element.identity.key === session.selection?.identity.key
    || element.identity.selector === session.selection?.identity.selector
  ))?.styleDiff ?? {}
}

function changedCount(session: PwaDesignSession, properties: PwaStyleProperty[]): number {
  const draftStyles = currentDraftStyles(session)
  return properties.filter((property) => Object.prototype.hasOwnProperty.call(draftStyles, property)).length
}

function StyleSection({
  title,
  hint,
  icon: Icon,
  count,
  defaultOpen = true,
  children,
}: {
  title: string
  hint: string
  icon: LucideIcon
  count: number
  defaultOpen?: boolean
  children: ReactNode
}) {
  const [open, setOpen] = useState(defaultOpen)
  return (
    <details
      className={styles.styleSection}
      open={open}
      onToggle={(event) => setOpen(event.currentTarget.open)}
    >
      <summary>
        <Icon size={15} />
        <span className={styles.sectionTitle}><strong>{title}</strong><small>{hint}</small></span>
        {count > 0 && <span className={styles.changeBadge}>{count} 项已改</span>}
        <ChevronDown className={styles.sectionChevron} size={15} />
      </summary>
      <div className={styles.sectionBody}>{children}</div>
    </details>
  )
}

function QuickAdjustments({ session }: Props) {
  return (
    <details className={styles.quickPanel}>
      <summary><Sparkles size={15} /><span>快速样式</span><small>常用预设</small><ChevronDown size={14} /></summary>
      <div className={styles.quickBody}>
        <div className={legacyStyles.pwaPresetGrid}>
          {STYLE_PRESETS.map((preset) => (
            <button
              key={preset.label}
              type="button"
              title={preset.hint}
              onClick={() => session.updateStyles(`preset:${preset.label}`, preset.styles)}
            >
              <strong>{preset.label}</strong><span>{preset.hint}</span>
            </button>
          ))}
          <button type="button" onClick={() => session.updateStyles('designer:primary', { backgroundColor: '#2563eb', color: '#ffffff', borderRadius: '14px' })}><strong>主按钮</strong><span>蓝底白字</span></button>
          <button type="button" onClick={() => session.updateStyles('designer:ghost', { backgroundColor: 'transparent', color: '#e5e7eb' })}><strong>透明底</strong><span>保留文字层级</span></button>
        </div>
      </div>
    </details>
  )
}

function OpacityControl({ session }: Props) {
  const rawValue = fieldValue(session, 'opacity') || '1'
  const numericValue = Number(rawValue)
  const safeValue = Number.isFinite(numericValue) ? Math.min(1, Math.max(0, numericValue)) : 1
  return (
    <label className={styles.opacityField}>
      <span className={styles.opacityHeader}>
        <span>整体透明度</span>
        <strong>{Math.round(safeValue * 100)}%</strong>
      </span>
      <span className={styles.opacityControl}>
        <input
          aria-label="整体透明度滑杆"
          type="range"
          min="0"
          max="1"
          step="0.01"
          value={safeValue}
          onChange={(event) => session.updateStyle('opacity', event.currentTarget.value)}
        />
        <input
          aria-label="整体透明度数值"
          value={rawValue}
          inputMode="decimal"
          onFocus={(event) => event.currentTarget.select()}
          onChange={(event) => session.updateStyle('opacity', event.currentTarget.value)}
        />
      </span>
      <small>作用于整个元素；颜色自身透明度请在颜色面板中调整。</small>
    </label>
  )
}

export function PwaStyleEditor({ session }: Props) {
  return (
    <div className={styles.editor} aria-label="选中组件样式编辑器">
      <QuickAdjustments session={session} />

      <StyleSection title="尺寸与布局" hint="支持 auto、%、rem、px" icon={Box} count={changedCount(session, LAYOUT_PROPERTIES)}>
        <div className={styles.twoColumn}>
          {SIZE_FIELDS.map((spec) => <StyleField key={spec.property} session={session} spec={spec} />)}
        </div>
      </StyleSection>

      <StyleSection title="间距" hint="内边距与外边距" icon={Move} count={changedCount(session, SPACING_PROPERTIES)}>
        <div className={styles.edgeGroup}>
          <div className={styles.subsectionTitle}><strong>内边距</strong><span>内容与边界</span></div>
          <EdgeFields session={session} fields={PADDING_FIELDS} />
        </div>
        <div className={styles.edgeGroup}>
          <div className={styles.subsectionTitle}><strong>外边距</strong><span>元素与周围</span></div>
          <EdgeFields session={session} fields={MARGIN_FIELDS} />
        </div>
      </StyleSection>

      <StyleSection title="形状与文字" hint="排版和圆角" icon={Type} count={changedCount(session, TYPE_PROPERTIES)}>
        <div className={styles.twoColumn}>
          {TYPE_FIELDS.map((spec) => <StyleField key={spec.property} session={session} spec={spec} />)}
        </div>
      </StyleSection>

      <StyleSection title="颜色与外观" hint="颜色和元素透明度" icon={Palette} count={changedCount(session, APPEARANCE_PROPERTIES)}>
        <div className={styles.colorStack}>
          <PwaColorStyleField session={session} property="color" label="文字颜色" value={fieldValue(session, 'color')} />
          <PwaColorStyleField session={session} property="backgroundColor" label="背景颜色" value={fieldValue(session, 'backgroundColor')} />
        </div>
        <OpacityControl session={session} />
      </StyleSection>

    </div>
  )
}

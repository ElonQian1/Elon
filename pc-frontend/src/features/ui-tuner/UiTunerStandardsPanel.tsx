import { ClipboardCopy, Database, GitBranch, Layers3 } from 'lucide-react'
import type { UiTunerElementStandard } from './types'
import type { UiTunerStandardInsight } from './standards'
import pageStyles from './UiTunerPage.module.css'
import styles from './UiTunerPanels.module.css'

interface UiTunerStandardsPanelProps {
  insight: UiTunerStandardInsight | null
  appliedStandard?: UiTunerElementStandard
  onApplyStandard: (standard: UiTunerElementStandard) => void
  onCopyStandardPackage: () => void
}

export function UiTunerStandardsPanel({
  insight,
  appliedStandard,
  onApplyStandard,
  onCopyStandardPackage,
}: UiTunerStandardsPanelProps) {
  if (!insight) return null

  const standard = appliedStandard ?? insight.standard

  return (
    <section className={pageStyles.section}>
      <div className={pageStyles.sectionHeader}>
        <h2>组件标准</h2>
        <button type="button" onClick={onCopyStandardPackage} aria-label="复制标准草案" title="复制标准草案">
          <ClipboardCopy size={14} aria-hidden="true" />
        </button>
      </div>

      <div className={styles.standardCard}>
        <div>
          <span>组件</span>
          <strong>{standard.component} · {standard.variant}</strong>
        </div>
        <div>
          <span>角色</span>
          <strong>{standard.role}</strong>
        </div>
        <div>
          <span>绑定</span>
          <strong>{confidenceLabel(insight.bindingConfidence)}</strong>
          <small>{insight.bindingReason}</small>
        </div>
      </div>

      <div className={styles.tokenGrid}>
        {Object.entries(standard.tokenRefs).map(([key, value]) => (
          <div key={key}>
            <span>{tokenLabel(key)}</span>
            <strong>{value}</strong>
          </div>
        ))}
      </div>

      <div className={styles.standardSplit}>
        <div>
          <Layers3 size={13} aria-hidden="true" />
          <span>{insight.reusableFields.join(' / ')}</span>
        </div>
        <div>
          <GitBranch size={13} aria-hidden="true" />
          <span>{insight.screenOnlyFields.join(' / ')}</span>
        </div>
      </div>

      <div className={styles.saveTargets}>
        {insight.saveTargets.map((target) => (
          <div key={target.id} className={target.recommended ? styles.recommendedTarget : ''}>
            <Database size={13} aria-hidden="true" />
            <div>
              <strong>{target.label}</strong>
              <span>{target.path}</span>
              <small>{target.intent}</small>
            </div>
          </div>
        ))}
      </div>

      <div className={pageStyles.inlineActions}>
        <button type="button" onClick={() => onApplyStandard(insight.standard)}>
          设为标准
        </button>
        <button type="button" onClick={onCopyStandardPackage}>
          复制草案
        </button>
      </div>
    </section>
  )
}

function confidenceLabel(confidence: UiTunerStandardInsight['bindingConfidence']) {
  if (confidence === 'high') return '高可信'
  if (confidence === 'medium') return '中可信'
  return '低可信'
}

function tokenLabel(key: string) {
  if (key === 'color') return '文字'
  if (key === 'background') return '背景'
  if (key === 'typography') return '字阶'
  if (key === 'spacing') return '间距'
  if (key === 'radius') return '圆角'
  return key
}

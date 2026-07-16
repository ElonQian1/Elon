import styles from './UiInspectorTabs.module.css'

export type UiInspectorTab = 'design' | 'ai' | 'inspect'

const TABS: Array<{ id: UiInspectorTab; label: string; hint: string }> = [
  { id: 'design', label: '设计', hint: '直接调整当前组件的尺寸、间距、文字和外观' },
  { id: 'ai', label: 'AI', hint: '让 AI 按需读取选区、设计图和源码绑定' },
  { id: 'inspect', label: '检查', hint: '查看 Android 节点、源码来源、坐标和导出数据' },
]

export function UiInspectorTabs({ value, onChange }: { value: UiInspectorTab; onChange: (tab: UiInspectorTab) => void }) {
  return (
    <div className={styles.tabs} role="tablist" aria-label="属性面板模式">
      {TABS.map((tab) => (
        <button
          key={tab.id}
          type="button"
          role="tab"
          aria-selected={value === tab.id}
          title={tab.hint}
          className={value === tab.id ? styles.active : ''}
          onClick={() => onChange(tab.id)}
        >
          {tab.label}
        </button>
      ))}
    </div>
  )
}

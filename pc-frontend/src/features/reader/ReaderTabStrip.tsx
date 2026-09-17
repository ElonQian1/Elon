import { useState, type MouseEvent } from 'react'
import { Columns2, ExternalLink, Maximize2, X } from 'lucide-react'
import ContextMenu, { type ContextMenuItem } from './ContextMenu'
import { badgeFor, type ReaderTab } from './readerTabsModel'
import { closeTab, dockTab, focusPopout, navigateTab, popOutTab } from './readerNative'
import { useReaderTabs } from './readerTabsStore'
import styles from './ReaderTabs.module.css'

/** Chrome-style strip: the workbench itself is the pinned first tab, reading tabs follow. */
export default function ReaderTabStrip() {
  const { tabs, activeId, presented, layout, activate, setLayout, setPresented } = useReaderTabs()
  const [menu, setMenu] = useState<{ x: number; y: number; tab: ReaderTab } | null>(null)
  if (tabs.length === 0) return null
  const workbenchActive = !presented
  const select = (tab: ReaderTab) => {
    if (tab.hosted === 'popout') { void focusPopout(tab.id); return }
    activate(tab.id)
  }
  const middleClose = (event: MouseEvent, tab: ReaderTab) => { if (event.button === 1) { event.preventDefault(); void closeTab(tab.id) } }
  const items = (tab: ReaderTab): ContextMenuItem[] => [
    { label: layout === 'docked' ? '✓ 停靠在聊天右侧' : '停靠在聊天右侧', onSelect: () => { setLayout('docked'); if (tab.hosted === 'main') activate(tab.id) } },
    { label: layout === 'overlay' ? '✓ 覆盖整个工作区' : '覆盖整个工作区', onSelect: () => { setLayout('overlay'); if (tab.hosted === 'main') activate(tab.id) } },
    tab.hosted === 'popout'
      ? { label: '收回到主窗口', onSelect: () => { void dockTab(tab.id).then(() => activate(tab.id)) } }
      : { label: '弹出为独立窗口', onSelect: () => { void popOutTab(tab.id) } },
    { label: '在系统浏览器打开', onSelect: () => { void navigateTab(tab.id, 'external') } },
    { label: '关闭标签', onSelect: () => { void closeTab(tab.id) }, danger: true },
    { label: '关闭其他标签', disabled: tabs.length < 2, onSelect: () => { tabs.filter(t => t.id !== tab.id).forEach(t => { void closeTab(t.id) }) } },
  ]
  return (
    <div className={styles.strip} role="tablist" aria-label="阅读标签">
      <button type="button" role="tab" className={styles.tab} data-active={workbenchActive} onClick={() => setPresented(false)}>
        <span className={styles.badge} data-kind="home">龙</span><span className={styles.label}>工作台</span>
      </button>
      {tabs.map(tab => {
        const active = presented && tab.id === activeId
        return (
          <div key={tab.id} role="tab" aria-selected={active} className={styles.tab} data-active={active} data-popout={tab.hosted === 'popout' || undefined}
            title={tab.title} onClick={() => select(tab)} onAuxClick={event => middleClose(event, tab)}
            onContextMenu={event => { event.preventDefault(); setMenu({ x: event.clientX, y: event.clientY, tab }) }}>
            <span className={styles.badge} data-loading={tab.loading || undefined}>{badgeFor(tab.site)}</span>
            <span className={styles.label}>{tab.title}</span>
            {tab.hosted === 'popout' && <ExternalLink size={11} aria-label="已弹出为独立窗口" />}
            <button type="button" className={styles.close} aria-label={`关闭 ${tab.title}`} onClick={event => { event.stopPropagation(); void closeTab(tab.id) }}><X size={12} /></button>
          </div>
        )
      })}
      <div className={styles.spacer} />
      <button type="button" className={styles.action} title={layout === 'docked' ? '当前：停靠在聊天右侧（点击切换为覆盖）' : '当前：覆盖整个工作区（点击切换为停靠）'}
        onClick={() => setLayout(layout === 'docked' ? 'overlay' : 'docked')}>
        {layout === 'docked' ? <Columns2 size={14} /> : <Maximize2 size={14} />}
      </button>
      {menu && <ContextMenu x={menu.x} y={menu.y} items={items(menu.tab)} onClose={() => setMenu(null)} />}
    </div>
  )
}

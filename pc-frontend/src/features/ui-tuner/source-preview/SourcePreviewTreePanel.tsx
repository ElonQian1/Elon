import { ChevronRight, CodeXml, EyeOff } from 'lucide-react'
import { memo, useCallback, useEffect, useRef } from 'react'
import { getSourcePreviewNodeLabels } from './sourcePreviewLabels'
import type { SourcePreviewNode } from './types'
import styles from './SourcePreview.module.css'

interface Props { root: SourcePreviewNode | null; selectedKey: string | null; onSelect: (key: string) => void }

export function SourcePreviewTreePanel({ root, selectedKey, onSelect }: Props) {
  const rows = useRef(new Map<string, HTMLButtonElement>())
  const selectedRow = useRef<HTMLButtonElement | null>(null)
  const registerRow = useCallback((key: string, element: HTMLButtonElement | null) => {
    if (element) rows.current.set(key, element)
    else rows.current.delete(key)
  }, [])

  useEffect(() => {
    if (selectedRow.current) {
      selectedRow.current.classList.remove(styles.selectedTreeRow)
      selectedRow.current.removeAttribute('aria-current')
    }
    const next = selectedKey ? rows.current.get(selectedKey) ?? null : null
    if (next) {
      next.classList.add(styles.selectedTreeRow)
      next.setAttribute('aria-current', 'true')
    }
    selectedRow.current = next
  }, [selectedKey, root])

  return (
    <aside className={styles.treePanel}>
      <header><CodeXml size={17} /><div><strong>源码组件树</strong><small>XML / 资源实时解析</small></div></header>
      <div className={styles.treeBody}>{root ? <StaticTreeNode node={root} depth={0} onSelect={onSelect} registerRow={registerRow} /> : <p>加载源码后显示真实父子层级。</p>}</div>
      <footer>截图与 XML 捕获仍保留在“真机证据”模式，不参与动态组件渲染。</footer>
    </aside>
  )
}

interface TreeNodeProps {
  node: SourcePreviewNode
  depth: number
  onSelect: (key: string) => void
  registerRow: (key: string, element: HTMLButtonElement | null) => void
}

const StaticTreeNode = memo(function StaticTreeNode({ node, depth, onSelect, registerRow }: TreeNodeProps) {
  const labels = getSourcePreviewNodeLabels(node)
  const indent = 10 + Math.min(depth, 8) * 12
  return <>
    <button
      ref={(element) => registerRow(node.key, element)}
      className={styles.treeRow}
      style={{ paddingLeft: indent }}
      onClick={() => onSelect(node.key)}
      title={labels.tooltip}
      aria-label={`${labels.primary}，${labels.type}`}
    >
      {node.children.length ? <ChevronRight size={13} /> : <span className={styles.treeIndent} />}
      {!node.style.visible && <EyeOff size={12} />}
      <span className={styles.treeNodeName}>{labels.primary}</span><small>{labels.type}</small>
    </button>
    {node.children.map((child) => <StaticTreeNode key={child.key} node={child} depth={depth + 1} onSelect={onSelect} registerRow={registerRow} />)}
  </>
})

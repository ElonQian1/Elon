import { ChevronRight, CodeXml, EyeOff } from 'lucide-react'
import { getSourcePreviewNodeLabels } from './sourcePreviewLabels'
import type { SourcePreviewNode } from './types'
import styles from './SourcePreview.module.css'

interface Props { root: SourcePreviewNode | null; selectedKey: string | null; onSelect: (key: string) => void }

export function SourcePreviewTreePanel({ root, selectedKey, onSelect }: Props) {
  return (
    <aside className={styles.treePanel}>
      <header><CodeXml size={17} /><div><strong>源码组件树</strong><small>XML / 资源实时解析</small></div></header>
      <div className={styles.treeBody}>{root ? <TreeNode node={root} depth={0} selectedKey={selectedKey} onSelect={onSelect} /> : <p>加载源码后显示真实父子层级。</p>}</div>
      <footer>截图与 XML 捕获仍保留在“真机证据”模式，不参与动态组件渲染。</footer>
    </aside>
  )
}

function TreeNode({ node, depth, selectedKey, onSelect }: { node: SourcePreviewNode; depth: number; selectedKey: string | null; onSelect: (key: string) => void }) {
  const labels = getSourcePreviewNodeLabels(node)
  const indent = 10 + Math.min(depth, 8) * 12
  return <>
    <button
      className={`${styles.treeRow} ${selectedKey === node.key ? styles.selectedTreeRow : ''}`}
      style={{ paddingLeft: indent }}
      onClick={() => onSelect(node.key)}
      title={labels.tooltip}
      aria-label={`${labels.primary}，${labels.type}`}
    >
      {node.children.length ? <ChevronRight size={13} /> : <span className={styles.treeIndent} />}
      {!node.style.visible && <EyeOff size={12} />}
      <span className={styles.treeNodeName}>{labels.primary}</span><small>{labels.type}</small>
    </button>
    {node.children.map((child) => <TreeNode key={child.key} node={child} depth={depth + 1} selectedKey={selectedKey} onSelect={onSelect} />)}
  </>
}

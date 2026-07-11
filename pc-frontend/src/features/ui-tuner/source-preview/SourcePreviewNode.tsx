import type { CSSProperties, MouseEvent } from 'react'
import type { SourcePreviewNode as PreviewNode } from './types'
import styles from './SourcePreview.module.css'

interface Props { node: PreviewNode; selectedKey: string | null; onSelect: (key: string) => void }

function dimension(value: string, weight: number): string | number | undefined {
  if (value === 'match_parent' || value === 'fill_parent') return '100%'
  if (value === 'wrap_content' || !value) return undefined
  if (value === '0dp' && weight > 0) return 0
  const number = Number.parseFloat(value)
  return Number.isFinite(number) ? number : undefined
}
function alignment(gravity: string): Pick<CSSProperties, 'alignItems' | 'justifyContent' | 'textAlign'> {
  return {
    alignItems: gravity.includes('center_horizontal') || gravity === 'center' ? 'center' : gravity.includes('end') || gravity.includes('right') ? 'flex-end' : 'stretch',
    justifyContent: gravity.includes('center_vertical') || gravity === 'center' ? 'center' : gravity.includes('bottom') ? 'flex-end' : 'flex-start',
    textAlign: gravity.includes('center') ? 'center' : gravity.includes('end') || gravity.includes('right') ? 'right' : 'left',
  }
}

export function SourcePreviewNode({ node, selectedKey, onSelect }: Props) {
  if (!node.style.visible) return null
  const isContainer = node.children.length > 0 || node.kind === 'group' || node.kind === 'list'
  const Element = node.kind === 'button' ? 'button' : node.kind === 'input' ? 'input' : 'div'
  const style: CSSProperties = {
    display: isContainer ? (node.layout.mode === 'stack' ? 'grid' : 'flex') : node.kind === 'spacer' ? 'block' : 'flex',
    flexDirection: node.layout.orientation,
    width: dimension(node.layout.width, node.layout.weight),
    height: dimension(node.layout.height, node.layout.weight),
    flexGrow: node.layout.weight || undefined,
    minWidth: node.kind === 'text' ? 4 : undefined,
    minHeight: node.kind === 'text' ? node.style.fontSize * 1.35 : 8,
    margin: `${node.layout.margin.top}px ${node.layout.margin.end}px ${node.layout.margin.bottom}px ${node.layout.margin.start}px`,
    padding: `${node.layout.padding.top}px ${node.layout.padding.end}px ${node.layout.padding.bottom}px ${node.layout.padding.start}px`,
    gap: node.layout.gap,
    color: node.style.textColor,
    background: node.style.background,
    fontSize: node.style.fontSize || undefined,
    fontWeight: node.style.fontWeight,
    borderRadius: node.style.borderRadius,
    opacity: node.style.opacity,
    ...alignment(node.layout.gravity),
  }
  if (node.layout.mode === 'stack') Object.assign(style, { gridTemplateAreas: '"stack"' })
  const handleClick = (event: MouseEvent) => { event.preventDefault(); event.stopPropagation(); onSelect(node.key) }
  const className = [styles.previewNode, styles[`kind_${node.kind}`], node.key === selectedKey ? styles.selectedNode : '', node.layout.mode === 'stack' ? styles.stackNode : ''].join(' ')
  if (node.kind === 'input') return <input className={className} style={style} value={node.style.text} readOnly onClick={handleClick} aria-label={node.name} />
  return (
    <Element type={Element === 'button' ? 'button' : undefined} className={className} style={style} onClick={handleClick} data-source-node={node.key}>
      {node.kind === 'image' && <span className={styles.imagePlaceholder}>图像</span>}
      {!isContainer && node.kind !== 'image' && node.style.text}
      {node.children.map((child) => <SourcePreviewNode key={child.key} node={child} selectedKey={selectedKey} onSelect={onSelect} />)}
    </Element>
  )
}

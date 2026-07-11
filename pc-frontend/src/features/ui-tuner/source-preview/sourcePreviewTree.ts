import type { SourcePreviewNode, SourcePreviewPatch } from './types'

export function findSourceNode(root: SourcePreviewNode | null, key: string | null): SourcePreviewNode | null {
  if (!root || !key) return null
  if (root.key === key) return root
  for (const child of root.children) {
    const match = findSourceNode(child, key)
    if (match) return match
  }
  return null
}
export function updateSourceNode(root: SourcePreviewNode, patch: SourcePreviewPatch): SourcePreviewNode {
  if (root.key === patch.nodeKey) return applyPatch(root, patch)
  let changed = false
  const children = root.children.map((child) => {
    const next = updateSourceNode(child, patch)
    changed ||= next !== child
    return next
  })
  return changed ? { ...root, children } : root
}

function applyPatch(node: SourcePreviewNode, patch: SourcePreviewPatch): SourcePreviewNode {
  const numeric = Number(patch.value)
  switch (patch.property) {
    case 'text': case 'textColor': case 'background':
      return { ...node, style: { ...node.style, [patch.property]: String(patch.value) } }
    case 'fontSize': case 'opacity': case 'borderRadius':
      return { ...node, style: { ...node.style, [patch.property]: Number.isFinite(numeric) ? numeric : 0 } }
    case 'width': case 'height': case 'gravity':
      return { ...node, layout: { ...node.layout, [patch.property]: String(patch.value) } }
    case 'paddingStart': case 'paddingTop': case 'paddingEnd': case 'paddingBottom':
      return changeEdge(node, 'padding', patch.property.slice(7).toLowerCase(), numeric)
    case 'marginStart': case 'marginTop': case 'marginEnd': case 'marginBottom':
      return changeEdge(node, 'margin', patch.property.slice(6).toLowerCase(), numeric)
    default:
      return node
  }
}

function changeEdge(node: SourcePreviewNode, group: 'padding' | 'margin', edge: string, value: number): SourcePreviewNode {
  const safeEdge = edge as keyof SourcePreviewNode['layout']['padding']
  return {
    ...node,
    layout: {
      ...node.layout,
      [group]: { ...node.layout[group], [safeEdge]: Number.isFinite(value) ? value : 0 },
    },
  }
}

export function flattenSourceTree(root: SourcePreviewNode): SourcePreviewNode[] {
  return [root, ...root.children.flatMap(flattenSourceTree)]
}

export function sourcePatchValue(property: string, value: string | number | boolean): string {
  if (['fontSize'].includes(property)) return `${value}sp`
  if (property.startsWith('padding') || property.startsWith('margin') || property === 'borderRadius') return `${value}dp`
  if ((property === 'width' || property === 'height') && /^-?\d+(\.\d+)?$/.test(String(value))) return `${value}dp`
  return String(value)
}

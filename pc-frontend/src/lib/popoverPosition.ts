export type PopoverAnchor = { top: number; right: number; bottom: number; left: number }

export const DEFAULT_POPOVER_ANCHOR: PopoverAnchor = { top: 200, right: 0, bottom: 238, left: 0 }

export function popoverAnchorFromRect(rect: Pick<DOMRect, 'top' | 'right' | 'bottom' | 'left'>): PopoverAnchor {
  return { top: rect.top, right: rect.right, bottom: rect.bottom, left: rect.left }
}

export function popoverAnchorFromPoint(x: number, y: number): PopoverAnchor {
  return { top: y, right: x, bottom: y + 1, left: x }
}

export function fixedPopoverPosition(anchor: PopoverAnchor, width: number, height: number, gap = 8) {
  const viewW = window.innerWidth
  const viewH = window.innerHeight
  const margin = 12
  const anchorCenterY = anchor.top + (anchor.bottom - anchor.top) / 2
  const maxTop = Math.max(margin, viewH - height - margin)
  const leftSide = anchor.left - width - gap
  const rightSide = anchor.right + gap
  const maxLeft = Math.max(gap, viewW - width - gap)
  return {
    top: Math.min(Math.max(anchorCenterY - 20, margin), maxTop),
    left: Math.min(Math.max(leftSide >= gap ? leftSide : rightSide, gap), maxLeft),
  }
}

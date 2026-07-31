import type { CSSProperties } from 'react'

type ActionTone = 'danger' | 'icon' | 'primary' | 'secondary'
type BadgeTone = 'danger' | 'neutral' | 'warn'

const actionBase: CSSProperties = {
  display: 'inline-flex',
  alignItems: 'center',
  justifyContent: 'center',
  gap: 6,
  minHeight: 30,
  padding: '0 10px',
  border: '1px solid var(--line)',
  borderRadius: 6,
  background: '#292a30',
  color: 'var(--text)',
  fontSize: 11,
  cursor: 'pointer',
}

const badgeBase: CSSProperties = {
  display: 'inline-flex',
  alignItems: 'center',
  width: 'fit-content',
  padding: '4px 7px',
  border: '1px solid #47675f',
  borderRadius: 999,
  color: '#a9ded2',
  fontSize: 9,
}

export const commerceStyles = {
  workspaceHeader: {
    borderColor: 'var(--line-soft)',
    borderRadius: 8,
    background: '#202126',
  },
  tabs: {
    display: 'flex',
    gap: 4,
    padding: 4,
    overflowX: 'auto',
    border: '1px solid var(--line-soft)',
    borderRadius: 8,
    background: '#1c1d21',
  },
  headerActions: {
    display: 'flex',
    alignItems: 'center',
    flexWrap: 'wrap',
    gap: 8,
  },
  grid: {
    display: 'grid',
    gridTemplateColumns: 'repeat(auto-fit, minmax(min(320px, 100%), 1fr))',
    gap: 8,
  },
  sectionBody: {
    border: 0,
    borderRadius: 0,
    background: 'transparent',
  },
  wideField: {
    gridColumn: '1 / -1',
  },
  checkRow: {
    display: 'flex',
    alignItems: 'center',
    gap: 7,
    minHeight: 30,
    color: 'var(--text-muted)',
    fontSize: 11,
  },
  list: {
    display: 'grid',
    gap: 7,
  },
  itemHeader: {
    display: 'flex',
    alignItems: 'center',
    justifyContent: 'space-between',
    gap: 9,
  },
  itemTitle: {
    margin: 0,
    overflowWrap: 'anywhere',
    fontSize: 12,
  },
  itemText: {
    margin: 0,
    color: 'var(--text-muted)',
    fontSize: 10,
  },
  itemMeta: {
    color: 'var(--text-muted)',
    fontSize: 10,
  },
  priorityRow: {
    display: 'grid',
    gridTemplateColumns: '22px minmax(0, 1fr) auto',
    alignItems: 'center',
    gap: 8,
    padding: 6,
    border: '1px solid var(--line)',
    borderRadius: 6,
  },
  priorityIndex: {
    color: '#d5b875',
    fontSize: 10,
    textAlign: 'center',
  },
  scrollArea: {
    maxHeight: 430,
    overflow: 'auto',
  },
  message: {
    padding: '10px 12px',
    borderLeft: '3px solid #47675f',
    fontSize: 11,
  },
} satisfies Record<string, CSSProperties>

export function tabStyle(active: boolean): CSSProperties {
  return {
    ...actionBase,
    flexShrink: 0,
    minWidth: 118,
    minHeight: 32,
    borderColor: active ? '#3e7e72' : 'transparent',
    background: active ? '#203b37' : 'transparent',
    color: active ? '#bde8df' : 'var(--text-muted)',
  }
}

export function actionStyle(tone: ActionTone, disabled = false): CSSProperties {
  const toneStyle: Record<ActionTone, CSSProperties> = {
    danger: {
      borderColor: '#854b50',
      background: '#482c31',
      color: '#ffc7cc',
    },
    icon: {
      width: 30,
      padding: 0,
    },
    primary: {
      borderColor: '#4aa28f',
      background: '#2f7467',
      color: '#fff',
    },
    secondary: {},
  }
  return {
    ...actionBase,
    ...toneStyle[tone],
    opacity: disabled ? 0.45 : 1,
    cursor: disabled ? 'not-allowed' : 'pointer',
  }
}

export function badgeStyle(tone: BadgeTone = 'neutral'): CSSProperties {
  const toneStyle: Record<BadgeTone, CSSProperties> = {
    danger: { borderColor: '#75464c', color: '#f0aeb5' },
    neutral: {},
    warn: { borderColor: '#755c35', color: '#f0c982' },
  }
  return { ...badgeBase, ...toneStyle[tone] }
}

export function listItemStyle(selected = false): CSSProperties {
  return {
    padding: 10,
    borderColor: selected ? '#4d8f82' : 'var(--line)',
    borderRadius: 7,
    background: selected ? '#1d302d' : '#191a1e',
    color: 'var(--text)',
    textAlign: 'left',
    font: 'inherit',
  }
}

export const errorMessageStyle: CSSProperties = {
  borderColor: '#75464c',
  color: '#ffc3c8',
}

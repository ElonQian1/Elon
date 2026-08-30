import type { LucideIcon } from 'lucide-react'
import {
  Activity,
  Banknote,
  Bot,
  Boxes,
  CircleCheck,
  CircleDollarSign,
  CircleMinus,
  ClipboardCheck,
  FileCheck2,
  Gauge,
  Gavel,
  GitBranch,
  Globe2,
  HardDrive,
  Landmark,
  LayoutDashboard,
  LockKeyhole,
  MessageCircleMore,
  MessagesSquare,
  Mic2,
  MonitorCog,
  Network,
  PackageCheck,
  Radar,
  ReceiptText,
  Scale,
  Settings2,
  ShieldCheck,
  SlidersHorizontal,
  TerminalSquare,
  TrendingUp,
  UsersRound,
} from 'lucide-react'

export type WorkspaceKey = 'ai' | 'projects' | 'messages' | 'compute' | 'admin'

export interface RailItem {
  path: string
  Icon: LucideIcon
  label: string
  workspace: WorkspaceKey
  color: string
  hoverColor: string
}

export interface NavItem {
  path: string
  label: string
  Icon: LucideIcon
}

export interface NavSection {
  id: string
  label: string
  items: NavItem[]
}

const colors = {
  neutral: { color: '#2a2b2f', hoverColor: '#34363b' },
  green: { color: '#26342f', hoverColor: '#315046' },
  blue: { color: '#28343a', hoverColor: '#34464f' },
  gold: { color: '#302f37', hoverColor: '#403e4b' },
} as const

export const WORKSPACE_RAIL_ITEMS: RailItem[] = [
  { path: '/projects', Icon: Boxes, label: '项目', workspace: 'projects', ...colors.neutral },
  { path: '/friends', Icon: MessagesSquare, label: '消息', workspace: 'messages', ...colors.neutral },
  { path: '/compute-market', Icon: Gauge, label: '算力', workspace: 'compute', ...colors.blue },
]

export const ADMIN_RAIL_ITEM: RailItem = {
  path: '/compute-activation',
  Icon: Settings2,
  label: '管理',
  workspace: 'admin',
  ...colors.gold,
}

export const LOCAL_RAIL_ITEMS: RailItem[] = [
  { path: '/local-tasks', Icon: HardDrive, label: '本机任务', workspace: 'projects', ...colors.green },
  { path: '/codex-control', Icon: TerminalSquare, label: 'Codex 控制台', workspace: 'projects', ...colors.green },
  { path: '/ai', Icon: Bot, label: '一龙 AI', workspace: 'ai', ...colors.green },
  { path: '/user-browser', Icon: Globe2, label: '官方 AI', workspace: 'ai', ...colors.green },
]

const aiSections: NavSection[] = [
  {
    id: 'ai-conversations',
    label: '对话',
    items: [
      { path: '/ai', Icon: Bot, label: '一龙 AI' },
      { path: '/ai-work-summary', Icon: ClipboardCheck, label: '工作摘要' },
    ],
  },
  {
    id: 'ai-providers',
    label: 'AI 提供方',
    items: [
      { path: '/user-browser', Icon: Globe2, label: '官方 AI' },
      { path: '/chatkit', Icon: MessageCircleMore, label: 'OpenAI ChatKit' },
    ],
  },
  {
    id: 'ai-tools',
    label: '工具',
    items: [
      { path: '/voice', Icon: Mic2, label: 'TTS 声音控制台' },
      { path: '/doctor', Icon: CircleCheck, label: '电脑医生' },
    ],
  },
]

const projectSections: NavSection[] = [
  {
    id: 'project-home',
    label: '项目',
    items: [
      { path: '/projects', Icon: LayoutDashboard, label: '项目中心' },
      { path: '/workspace', Icon: Bot, label: '项目对话' },
      { path: '/dev-tasks', Icon: Activity, label: '开发任务' },
    ],
  },
  {
    id: 'project-development',
    label: '开发',
    items: [
      { path: '/git-worktrees', Icon: GitBranch, label: 'Git 现场' },
      { path: '/codex-control', Icon: TerminalSquare, label: 'Codex 控制台' },
      { path: '/local-tasks', Icon: HardDrive, label: '本机任务' },
    ],
  },
  {
    id: 'project-delivery',
    label: '设计与交付',
    items: [
      { path: '/ui-tuner', Icon: SlidersHorizontal, label: '微调画布' },
    ],
  },
]

const messageSections: NavSection[] = [
  {
    id: 'messages-home',
    label: '消息',
    items: [
      { path: '/friends', Icon: UsersRound, label: '好友' },
    ],
  },
]

const computeSections: NavSection[] = [
  {
    id: 'compute-overview',
    label: '总览',
    items: [
      { path: '/compute-market', Icon: LayoutDashboard, label: '算力总览' },
      { path: '/compute-execution', Icon: Activity, label: '当前执行' },
      { path: '/my-compute-settlement', Icon: CircleDollarSign, label: '我的收益' },
    ],
  },
  {
    id: 'compute-resources',
    label: '资源与使用',
    items: [
      { path: '/node', Icon: MonitorCog, label: '我的节点' },
      { path: '/compute-supply', Icon: Gauge, label: '供给管理' },
      { path: '/compute-external-pools', Icon: Network, label: '外部算力池' },
      { path: '/compute-reviews', Icon: ClipboardCheck, label: '算力验收' },
    ],
  },
  {
    id: 'compute-settlement',
    label: '结算',
    items: [
      { path: '/compute-challenges', Icon: Scale, label: '结算申诉' },
      { path: '/compute-settlement', Icon: Landmark, label: '结算记录' },
    ],
  },
]

const adminSections: NavSection[] = [
  {
    id: 'admin-governance',
    label: '算力治理',
    items: [
      { path: '/compute-activation', Icon: FileCheck2, label: '激活审核' },
      { path: '/compute-offers', Icon: PackageCheck, label: 'Offer 管理' },
      { path: '/compute-reference-curves', Icon: TrendingUp, label: '平台参考价格' },
      { path: '/compute-observations', Icon: Radar, label: '平台观测' },
      { path: '/compute-verification', Icon: ShieldCheck, label: '算力验证' },
    ],
  },
  {
    id: 'admin-settlement',
    label: '终态与结算',
    items: [
      { path: '/compute-receipts', Icon: ReceiptText, label: '执行回执' },
      { path: '/compute-finalization', Icon: LockKeyhole, label: '可信终态' },
      { path: '/compute-settlement-issuance', Icon: Banknote, label: '待结算回执' },
      { path: '/compute-challenge-resolution', Icon: Gavel, label: '申诉裁决' },
      { path: '/compute-corrections', Icon: CircleMinus, label: '结算纠正' },
    ],
  },
]

// These routes already own their contextual navigation. Rendering the global
// workspace nav alongside them creates the same two-level sidebar that the
// shell is intended to eliminate.
const CONTEXTUAL_NAV_PATHS = [
  '/ai',
  '/workspace',
  '/friends',
  '/projects',
  '/plaza',
  '/compute-market',
  '/compute-supply',
  '/compute-activation',
  '/compute-offers',
  '/compute-reference-curves',
  '/local-tasks',
  '/codex-control',
  '/doctor',
  '/node',
] as const

export function sectionsForWorkspace(workspace: WorkspaceKey): NavSection[] {
  switch (workspace) {
    case 'ai': return aiSections
    case 'projects': return projectSections
    case 'messages': return messageSections
    case 'compute': return computeSections
    case 'admin': return adminSections
  }
}

export function workspaceForPath(pathname: string): WorkspaceKey {
  if (pathname.startsWith('/friends') || pathname.startsWith('/users')) return 'messages'
  if (pathname.startsWith('/compute-') || pathname.startsWith('/my-compute-') || pathname === '/node') {
    const adminPath = ['/compute-activation', '/compute-offers', '/compute-reference-curves', '/compute-observations',
      '/compute-verification', '/compute-receipts', '/compute-finalization', '/compute-settlement-issuance',
      '/compute-challenge-resolution', '/compute-corrections']
    return adminPath.some((path) => pathname.startsWith(path)) ? 'admin' : 'compute'
  }
  if (pathname.startsWith('/workspace') || pathname.startsWith('/projects') || pathname.startsWith('/git-worktrees')
    || pathname.startsWith('/ui-tuner') || pathname.startsWith('/local-tasks') || pathname.startsWith('/codex-control')
    || pathname.startsWith('/dev-tasks')) return 'projects'
  return 'ai'
}

export function pathMatches(pathname: string, path: string): boolean {
  if (path === '/ai') return pathname === '/ai'
  return pathname === path || pathname.startsWith(`${path}/`)
}

export function shouldShowWorkspaceNav(pathname: string): boolean {
  return !CONTEXTUAL_NAV_PATHS.some((path) => pathMatches(pathname, path))
}

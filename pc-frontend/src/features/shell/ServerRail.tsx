import { useNavigate, useLocation } from 'react-router-dom'
import { useState } from 'react'
import type { LucideIcon } from 'lucide-react'
import { Activity, Banknote, Bot, Boxes, CircleDollarSign, CircleMinus, ClipboardCheck, FileCheck2, Gauge, Gavel, GitBranch, Globe2, HardDrive, Landmark, LockKeyhole, MessageCircleMore, MonitorCog, PackageCheck, Radar, ReceiptText, Scale, Search, ShieldCheck, UsersRound, Mic2, SlidersHorizontal, TerminalSquare } from 'lucide-react'
import { useAuthStore } from '../../store/auth'
import { isLocalWorkbench } from '../../api/runtime'
import { useProjectStore } from '../conversation/useProjectStore'
import UserAvatar, { userDisplayName } from './UserAvatar'
import { presenceLabel, useMyPresence } from './useMyPresence'
import styles from './ServerRail.module.css'

interface RailItem {
  path: string
  Icon: LucideIcon
  label: string
  color: string
  hoverColor: string
}

const RAIL_ITEMS: RailItem[] = [
  { path: '/ai',      Icon: Bot,          label: '一龙 AI',   color: '#2a2b2f', hoverColor: '#34363b' },
  { path: '/projects', Icon: Boxes,       label: '项目中心',  color: '#2a2b2f', hoverColor: '#34363b' },
  { path: '/friends', Icon: UsersRound,   label: '好友',      color: '#2a2b2f', hoverColor: '#34363b' },
  { path: '/git-worktrees', Icon: GitBranch, label: 'Git 现场', color: '#2a2b2f', hoverColor: '#34363b' },
  { path: '/ui-tuner', Icon: SlidersHorizontal, label: '微调画布', color: '#2a2b2f', hoverColor: '#34363b' },
  { path: '/codex-control', Icon: TerminalSquare, label: 'Codex 控制台', color: '#26342d', hoverColor: '#30463a' },
  { path: '/node',    Icon: MonitorCog,   label: '分享算力',  color: '#2a2b2f', hoverColor: '#34363b' },
  { path: '/compute-supply', Icon: Gauge, label: '供给管理', color: '#2b3138', hoverColor: '#35404a' },
  { path: '/compute-market', Icon: Search, label: '算力市场', color: '#30312d', hoverColor: '#3b3d36' },
  { path: '/compute-reviews', Icon: ClipboardCheck, label: '算力验收', color: '#342f28', hoverColor: '#463d31' },
  { path: '/compute-execution', Icon: Activity, label: '算力执行', color: '#28343a', hoverColor: '#34464f' },
  { path: '/my-compute-settlement', Icon: CircleDollarSign, label: '我的算力收益', color: '#26342d', hoverColor: '#30463a' },
  { path: '/compute-challenges', Icon: Scale, label: '结算申诉', color: '#362d29', hoverColor: '#493a33' },
  { path: '/voice',   Icon: Mic2,         label: 'AI 声音',  color: '#2a2b2f', hoverColor: '#34363b' },
  { path: '/user-browser', Icon: Globe2, label: '官方 AI', color: '#26342f', hoverColor: '#315046' },
  { path: '/chatkit', Icon: MessageCircleMore, label: 'OpenAI ChatKit', color: '#26342f', hoverColor: '#315046' },
]

const LOCAL_TASK_ITEM: RailItem = {
  path: '/local-tasks', Icon: HardDrive, label: '本机任务', color: '#26342d', hoverColor: '#30463a',
}

const LOCAL_CODEX_CONTROL_ITEM: RailItem = {
  path: '/codex-control', Icon: TerminalSquare, label: 'Codex 控制台', color: '#26342d', hoverColor: '#30463a',
}

const SETTLEMENT_ITEM: RailItem = {
  path: '/compute-settlement', Icon: Landmark, label: '算力结算', color: '#2d3431', hoverColor: '#37433e',
}

const ACTIVATION_ITEM: RailItem = {
  path: '/compute-activation', Icon: FileCheck2, label: '算力激活审核', color: '#30333b', hoverColor: '#3b414d',
}

const OFFER_ITEM: RailItem = {
  path: '/compute-offers', Icon: PackageCheck, label: '算力 Offer 管理', color: '#33312c', hoverColor: '#454138',
}

const OBSERVATION_ITEM: RailItem = {
  path: '/compute-observations', Icon: Radar, label: '平台观测', color: '#28333a', hoverColor: '#344650',
}

const VERIFICATION_ITEM: RailItem = {
  path: '/compute-verification', Icon: ShieldCheck, label: '算力验证', color: '#29352f', hoverColor: '#35483e',
}

const RECEIPT_ITEM: RailItem = {
  path: '/compute-receipts', Icon: ReceiptText, label: '执行回执', color: '#373226', hoverColor: '#4a4331',
}

const FINALIZATION_ITEM: RailItem = {
  path: '/compute-finalization', Icon: LockKeyhole, label: '可信终态', color: '#3b2924', hoverColor: '#50362e',
}

const SETTLEMENT_ISSUANCE_ITEM: RailItem = {
  path: '/compute-settlement-issuance', Icon: Banknote, label: '待结算回执', color: '#393826', hoverColor: '#4d4b31',
}

const CHALLENGE_RESOLUTION_ITEM: RailItem = {
  path: '/compute-challenge-resolution', Icon: Gavel, label: '申诉裁决', color: '#3b2c28', hoverColor: '#503a34',
}

const SETTLEMENT_CORRECTION_ITEM: RailItem = {
  path: '/compute-corrections', Icon: CircleMinus, label: '结算纠正', color: '#3b292b', hoverColor: '#503438',
}

export default function ServerRail() {
  const navigate = useNavigate()
  const { pathname } = useLocation()
  const user = useAuthStore((s) => s.user)
  const localMode = isLocalWorkbench()
  const presence = useMyPresence(!localMode)
  const [tooltip, setTooltip] = useState<{ text: string; y: number } | null>(null)
  const railItems = user && ['admin', 'owner'].includes(user.role ?? '')
    ? [...RAIL_ITEMS, ACTIVATION_ITEM, OFFER_ITEM, OBSERVATION_ITEM, VERIFICATION_ITEM, RECEIPT_ITEM, FINALIZATION_ITEM, SETTLEMENT_ISSUANCE_ITEM, CHALLENGE_RESOLUTION_ITEM, SETTLEMENT_CORRECTION_ITEM, SETTLEMENT_ITEM]
    : RAIL_ITEMS

  // 项目列表（从 store 读取，实时响应）
  const projects = useProjectStore((s) => s.projects)
  const activeProjectId = useProjectStore((s) => s.activeProjectId)

  function isActive(path: string) {
    return pathname.startsWith(path)
  }

  function handleRailClick(path: string) {
    navigate(path)
  }

  async function openProject(id: string) {
    // 先更新状态（高亮立即生效），再导航到项目对话页
    await useProjectStore.getState().selectProject(id)
    if (pathname !== '/workspace') navigate('/workspace')
  }

  function showTip(e: React.MouseEvent<HTMLElement>, text: string) {
    const rect = (e.currentTarget as HTMLElement).getBoundingClientRect()
    setTooltip({ text, y: rect.top + rect.height / 2 })
  }

  return (
    <nav className={styles.rail}>
      {(localMode ? [LOCAL_TASK_ITEM, LOCAL_CODEX_CONTROL_ITEM] : railItems).map((item) => {
        const active = isActive(item.path)
        const Icon = item.Icon
        return (
          <button
            key={item.path}
            className={[styles.avatar, active ? styles.active : ''].join(' ')}
            style={{ '--item-color': item.color, '--item-hover': item.hoverColor } as React.CSSProperties}
            onClick={() => handleRailClick(item.path)}
            onMouseEnter={(e) => showTip(e, item.label)}
            onMouseLeave={() => setTooltip(null)}
            title={item.label}
            type="button"
          >
            <Icon className={styles.icon} aria-hidden="true" strokeWidth={2.3} />
          </button>
        )
      })}

      {/* ── 项目列表分隔线 ── */}
      {!localMode && projects.length > 0 && <div className={styles.divider} />}

      {!localMode && <div className={styles.projectStack} aria-label="项目快捷入口">
        {projects.map((p) => {
          const isActiveProject = pathname === '/workspace' && p.id === activeProjectId
          const iconSrc = p.icon_data_url || p.icon || ''
          return (
            <button
              key={p.id}
              className={[styles.avatar, styles.projectAvatar, isActiveProject ? styles.active : ''].join(' ')}
              onClick={() => openProject(p.id)}
              onMouseEnter={(e) => showTip(e, p.name)}
              onMouseLeave={() => setTooltip(null)}
              title={p.name}
              type="button"
            >
              {iconSrc
                ? <img src={iconSrc} alt="" className={styles.projectIcon} onError={(e) => { (e.currentTarget as HTMLImageElement).style.display = 'none' }} />
                : <span className={styles.projectFallback}>{p.name[0]?.toUpperCase() ?? '?'}</span>
              }
            </button>
          )
        })}
      </div>}

      {!localMode && <div className={styles.divider} />}

      {/* 账号头像 → 点击进账号页 */}
      {!localMode && user && (
        <button
          className={[styles.avatar, styles.userAvatar].join(' ')}
          title={`${userDisplayName(user)} — ${presenceLabel(presence?.status)}`}
          aria-label={`${userDisplayName(user)} — ${presenceLabel(presence?.status)}`}
          onMouseEnter={(e) => showTip(e, `${userDisplayName(user)} · ${presenceLabel(presence?.status)}`)}
          onMouseLeave={() => setTooltip(null)}
          onClick={() => navigate('/account')}
          type="button"
        >
          <UserAvatar user={user} size="rail" showStatus presenceStatus={presence?.status} className={styles.railUserAvatar} />
        </button>
      )}

      {/* Tooltip */}
      {tooltip && (
        <div className={styles.tooltip} style={{ top: tooltip.y, transform: 'translateY(-50%)' }}>
          {tooltip.text}
        </div>
      )}
    </nav>
  )
}

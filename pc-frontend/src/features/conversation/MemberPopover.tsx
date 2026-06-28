import { useEffect, useRef } from 'react'
import { useNavigate } from 'react-router-dom'
import type { ProjectMember } from './types'
import { formatTime } from '../../lib/utils'
import styles from './MemberPopover.module.css'

interface Props {
  member: ProjectMember
  anchorY: number   // viewport Y center of clicked row
  onClose: () => void
}

const ROLE_LABELS: Record<string, string> = {
  owner: '拥有者', admin: '管理员', editor: '协作者',
  collaborator: '协作者', member: '成员', observer: '只读成员',
}

const ROLE_HEAD_CLASS: Record<string, string> = {
  owner: styles.headOwner, admin: styles.headAdmin,
  editor: styles.headEditor, collaborator: styles.headEditor,
}

export default function MemberPopover({ member, anchorY, onClose }: Props) {
  const navigate = useNavigate()
  const popRef = useRef<HTMLDivElement>(null)

  // 点击外部关闭
  useEffect(() => {
    function onDown(e: MouseEvent) {
      if (popRef.current && !popRef.current.contains(e.target as Node)) {
        onClose()
      }
    }
    document.addEventListener('mousedown', onDown)
    return () => document.removeEventListener('mousedown', onDown)
  }, [onClose])

  const roleKey = (member.role ?? '').toLowerCase()
  const roleLabel = ROLE_LABELS[roleKey] ?? '成员'
  const isOnline = member.is_online
  const name = member.account ?? member.user_id
  const shortId = member.user_id?.slice(0, 12) ?? ''

  // 定位：出现在右侧栏左侧，Y 对齐点击行
  const popoverStyle: React.CSSProperties = {
    position: 'fixed',
    right: 280,        // 右侧栏 272px + 8px gap
    top: Math.min(Math.max(anchorY - 20, 12), window.innerHeight - 300),
    zIndex: 200,
  }

  function copyId() {
    navigator.clipboard.writeText(member.user_id).catch(() => {})
  }

  function sendMessage() {
    onClose()
    navigate('/friends')
  }

  return (
    <div ref={popRef} className={styles.popover} style={popoverStyle}>
      {/* 头部：头像 + 关闭 */}
      <div className={[styles.head, ROLE_HEAD_CLASS[roleKey] ?? ''].join(' ')}>
        <div className={[styles.avatar, isOnline ? styles.avatarOnline : styles.avatarOffline].join(' ')}>
          {member.avatar_data_url
            ? <img src={member.avatar_data_url} alt="" />
            : <span>{name[0]?.toUpperCase() ?? '?'}</span>
          }
        </div>
        <button className={styles.close} onClick={onClose} type="button" title="关闭">×</button>
      </div>

      {/* 主体 */}
      <div className={styles.body}>
        <strong className={styles.name}>{name}</strong>
        <span className={styles.sub}>{isOnline ? '在线' : '离线'}</span>

        {/* 标签行 */}
        <div className={styles.meta}>
          <em className={styles.pill}>{roleLabel}</em>
          <em className={[styles.pill, isOnline ? styles.pillOnline : styles.pillOffline].join(' ')}>
            {isOnline ? '在线' : '离线'}
          </em>
          {shortId && <em className={styles.pill}>{shortId}</em>}
        </div>

        {/* 详情 */}
        <div className={styles.details}>
          {member.account && (
            <div className={styles.detail}><span>账号</span><strong>{member.account}</strong></div>
          )}
          {member.user_id && (
            <div className={styles.detail}><span>用户 ID</span><strong title={member.user_id}>{shortId}</strong></div>
          )}
          {member.joined_at && (
            <div className={styles.detail}><span>加入时间</span><strong>{formatTime(member.joined_at)}</strong></div>
          )}
        </div>

        {/* 操作按钮 */}
        <div className={styles.actions}>
          <button className={styles.actionBtn} type="button" onClick={sendMessage}>
            发消息
          </button>
          <button className={styles.actionBtn} type="button" onClick={copyId}>
            复制 ID
          </button>
        </div>
      </div>
    </div>
  )
}

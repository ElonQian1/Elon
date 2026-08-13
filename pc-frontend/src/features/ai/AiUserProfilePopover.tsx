import { useEffect, useRef, useState } from 'react'
import { useNavigate } from 'react-router-dom'
import { api } from '../../api/client'
import { fixedPopoverPosition, type PopoverAnchor } from '../../lib/popoverPosition'

export interface AiChatFriend {
  id: string
  account: string
  nickname?: string
  avatar_data_url?: string | null
  is_online?: boolean
  already_friend?: boolean
}

export default function AiUserProfilePopover({ friend, anchor, onClose }: { friend: AiChatFriend; anchor: PopoverAnchor; onClose: () => void }) {
  const navigate = useNavigate()
  const popRef = useRef<HTMLDivElement>(null)
  const name = friend.nickname ?? friend.account
  const isOnline = !!friend.is_online
  const [isFriend, setIsFriend] = useState(!!friend.already_friend)
  const [addingFriend, setAddingFriend] = useState(false)
  const [addMsg, setAddMsg] = useState('')

  useEffect(() => {
    function onDown(e: MouseEvent) {
      if (popRef.current && !popRef.current.contains(e.target as Node)) onClose()
    }
    document.addEventListener('mousedown', onDown)
    return () => document.removeEventListener('mousedown', onDown)
  }, [onClose])

  const width = 284
  const { left, top } = fixedPopoverPosition(anchor, width, 280)

  async function addFriend() {
    if (isFriend || addingFriend) return
    setAddingFriend(true)
    try {
      await api.post('/api/me/friends', { query: friend.id, search_type: 'user_id' })
      setIsFriend(true)
      setAddMsg('已添加')
    } catch (err) {
      setAddMsg((err as { message?: string }).message ?? '添加失败')
    } finally {
      setAddingFriend(false)
    }
  }

  return (
    <div ref={popRef} style={{ position: 'fixed', left, top, zIndex: 9999, width, background: '#1e1f22', border: '1px solid rgba(255,255,255,.12)', borderRadius: 10, overflow: 'hidden', boxShadow: '0 8px 32px rgba(0,0,0,.55)' }}>
      <div style={{ position: 'relative', height: 72, background: isOnline ? 'linear-gradient(135deg,#0a2d1f,#0d2012)' : '#2c2e35' }}>
        <div style={{ position: 'absolute', bottom: -18, left: 14, width: 56, height: 56, borderRadius: '50%', border: '4px solid #1e1f22', background: '#38414a', display: 'grid', placeItems: 'center', overflow: 'hidden' }}>
          {friend.avatar_data_url
            ? <img src={friend.avatar_data_url} alt="" style={{ width: '100%', height: '100%', objectFit: 'cover', borderRadius: '50%' }} />
            : <span style={{ fontSize: 20, fontWeight: 800, color: 'white' }}>{name[0]?.toUpperCase()}</span>}
          <span style={{ position: 'absolute', right: 1, bottom: 1, width: 13, height: 13, borderRadius: '50%', border: '3px solid #1e1f22', background: isOnline ? 'var(--green,#58BE6A)' : '#545862' }} />
        </div>
        <button onClick={onClose} type="button" style={{ position: 'absolute', top: 8, right: 8, width: 28, height: 28, border: 0, borderRadius: '50%', background: 'rgba(0,0,0,.35)', color: '#c4c8d4', fontSize: 18, cursor: 'pointer', display: 'grid', placeItems: 'center' }}>×</button>
      </div>
      <div style={{ padding: '24px 14px 14px', display: 'flex', flexDirection: 'column', gap: 4 }}>
        <strong style={{ display: 'block', fontSize: 15, fontWeight: 800, color: 'var(--text)', marginTop: 6, overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap' }}>{name}</strong>
        <span style={{ fontSize: 12, color: 'var(--text-muted)' }}>{isOnline ? '在线' : '离线'}</span>
        <div style={{ display: 'flex', flexWrap: 'wrap', gap: 5, marginTop: 6 }}>
          <em style={{ display: 'inline-flex', alignItems: 'center', height: 17, padding: '0 6px', borderRadius: 4, border: '1px solid rgba(255,255,255,.1)', background: 'rgba(255,255,255,.04)', color: isOnline ? 'var(--green,#58BE6A)' : '#aab0bd', fontSize: 10, fontWeight: 800, fontStyle: 'normal' }}>{isOnline ? '在线' : '离线'}</em>
          {friend.id && <em style={{ display: 'inline-flex', alignItems: 'center', height: 17, padding: '0 6px', borderRadius: 4, border: '1px solid rgba(255,255,255,.1)', background: 'rgba(255,255,255,.04)', color: '#aab0bd', fontSize: 10, fontWeight: 800, fontStyle: 'normal' }}>{friend.id.slice(0, 7).toUpperCase()}</em>}
        </div>
        <div style={{ marginTop: 10, borderTop: '1px solid rgba(255,255,255,.06)', paddingTop: 8, display: 'flex', flexDirection: 'column', gap: 5 }}>
          {friend.account && <div style={{ display: 'flex', justifyContent: 'space-between', fontSize: 12, gap: 8 }}><span style={{ color: 'var(--text-muted)', flexShrink: 0 }}>账号</span><strong style={{ color: 'var(--text-soft)', fontWeight: 500, overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap', textAlign: 'right' }}>{friend.account}</strong></div>}
          {friend.id && <div style={{ display: 'flex', justifyContent: 'space-between', fontSize: 12, gap: 8 }}><span style={{ color: 'var(--text-muted)', flexShrink: 0 }}>用户 ID</span><strong style={{ color: 'var(--text-soft)', fontWeight: 500, overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap', textAlign: 'right' }} title={friend.id}>{friend.id.slice(0, 14).toUpperCase()}</strong></div>}
        </div>
        <div style={{ display: 'flex', gap: 6, marginTop: 10, flexWrap: 'wrap' }}>
          <button style={actionStyle} type="button" onClick={() => { onClose(); navigate('/friends') }}>发消息</button>
          <button style={{ ...actionStyle, background: isFriend ? 'rgba(88,190,106,.1)' : actionStyle.background, color: isFriend ? 'var(--green,#58BE6A)' : actionStyle.color, cursor: isFriend ? 'default' : 'pointer', opacity: addingFriend ? 0.6 : 1 }} type="button" onClick={addFriend} disabled={isFriend || addingFriend}>{addMsg || (isFriend ? '已是好友' : addingFriend ? '添加中…' : '加好友')}</button>
          <button style={actionStyle} type="button" onClick={() => navigator.clipboard.writeText(friend.id).catch(() => {})}>复制 ID</button>
        </div>
      </div>
    </div>
  )
}

const actionStyle = {
  flex: 1,
  height: 30,
  border: '1px solid rgba(255,255,255,.12)',
  borderRadius: 6,
  background: 'rgba(255,255,255,.04)',
  color: 'var(--text-soft)',
  fontSize: 12,
  fontWeight: 600,
  cursor: 'pointer',
} as const

import { useEffect, useState } from 'react'
import { api } from '../../api/client'
import SocialDialog from '../friends/SocialDialog'

interface Result { found?: boolean; user?: { id: string; nickname?: string; phone?: string }; already_friend?: boolean; is_self?: boolean }
export default function ScanFriendDialog({ id, onClose }: { id: string; onClose: () => void }) {
  const [result, setResult] = useState<Result | null>(null)
  const [notice, setNotice] = useState('正在查找账号…'), [busy, setBusy] = useState(false)
  useEffect(() => {
    let active = true
    void api.get<Result>(`/api/me/friends/search?search_type=account_id&query=${encodeURIComponent(id)}`).then(value => {
      if (!active) return
      setResult(value); setNotice(value.user ? value.is_self ? '这是你自己的账号' : value.already_friend ? '已是好友' : '确认账号后可添加好友' : '未找到注册账号，请让对方登录后重新出示二维码')
    }).catch(() => { if (active) setNotice('查找失败，请确认已登录并检查网络后重试') })
    return () => { active = false }
  }, [id])
  async function add() {
    if (busy || !result?.user) return
    setBusy(true)
    try { await api.post('/api/me/friends', { query: result.user.id, search_type: 'account_id' }); setResult({ ...result, already_friend: true }); setNotice('已添加好友，可在好友列表查看') }
    catch { setNotice('添加失败，请稍后重试') } finally { setBusy(false) }
  }
  return <SocialDialog title="查找扫码账号" onClose={onClose} busy={busy} footer={result?.user && !result.already_friend && !result.is_self ? <button type="button" disabled={busy} onClick={() => void add()}>添加好友</button> : undefined}>
    <p>{result?.user?.nickname || result?.user?.phone || id}</p><p role="status">{notice}</p>
  </SocialDialog>
}

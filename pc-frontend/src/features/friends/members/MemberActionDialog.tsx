import { useRef, useState } from 'react'
import { api } from '../../../api/client'
import SocialDialog from '../SocialDialog'
import { SOCIAL_REFRESH_EVENT } from '../socialRealtime'
import { memberError, type Command, type Receipt } from './rosterTypes'

export interface Confirmation { title: string; explanation: string; command: Command }
export default function MemberActionDialog({ groupId, confirmation, onClose, onDone }: {
  groupId: string; confirmation: Confirmation; onClose: () => void; onDone: (receipt: Receipt) => void
}) {
  const request = useRef(Array.from(crypto.getRandomValues(new Uint8Array(16)), byte => byte.toString(16).padStart(2, '0')).join(''))
  const [busy, setBusy] = useState(false)
  const [error, setError] = useState('')
  async function submit() {
    setBusy(true); setError('')
    try {
      const receipt = await api.post<Receipt>(`/api/me/groups/${encodeURIComponent(groupId)}/membership`, { ...confirmation.command, request_id: request.current })
      window.dispatchEvent(new Event(SOCIAL_REFRESH_EVENT)); onDone(receipt)
    } catch (reason) { setError(memberError(reason)) }
    finally { setBusy(false) }
  }
  return <SocialDialog title={confirmation.title} onClose={onClose} busy={busy}
    footer={<><button type="button" disabled={busy} onClick={onClose}>取消</button><button type="button" disabled={busy} onClick={() => void submit()}>{busy ? '正在处理…' : confirmation.title}</button></>}>
    <p>{confirmation.explanation}</p>{error && <p role="alert">{error}。可重试此操作。</p>}
  </SocialDialog>
}

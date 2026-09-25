import { useEffect, useRef, useState } from 'react'
import { Paperclip } from 'lucide-react'
import { getDesktopInvoke } from '../shell/desktopShell'
import { getLocalAiWebSessionState } from './localAiBrowserApi'
import { uploadPrivateAttachments } from './privateAttachmentUpload'
import type { AiWebChatBackend } from './useAiWebChatBackend'

export default function AiWebPrivateAttachmentButton({ web }: { web: AiWebChatBackend }) {
  const input = useRef<HTMLInputElement>(null), latest = useRef(web)
  latest.current = web
  const alive = useRef(true)
  const [busy, setBusy] = useState(false), [error, setError] = useState('')
  useEffect(() => { alive.current = true; return () => { alive.current = false } }, [])
  async function upload(files: File[]) {
    const identity = web.officialRequest
    if (!files.length || !identity || busy) return
    setBusy(true); setError('')
    const check = () => {
      if (!alive.current || latest.current.officialRequest?.ownerKey !== identity.ownerKey
        || latest.current.provider.id !== 'chatgpt') throw new Error('会话已切换，附件上传已停止')
    }
    try {
      const invoke = getDesktopInvoke()
      if (!invoke || web.provider.desktopRuntimeVersion < 15) throw new Error('请升级 Windows 客户端后使用私有附件上传')
      await uploadPrivateAttachments(files.map(file => ({ name: file.name, type: file.type, size: file.size, load: async () => file })), {
        command: (value, requestId) => invoke('run_local_ai_web_adapter_command', {
          providerId: 'chatgpt', ownerKey: identity.ownerKey, action: 'stage_attachments', value, requestId,
        }),
        state: () => getLocalAiWebSessionState('chatgpt', identity.ownerKey), check,
      })
      check()
      await web.controller.run('snapshot')
    } catch (failure) { if (alive.current) setError(failure instanceof Error ? failure.message : '附件上传失败') }
    finally { if (alive.current) setBusy(false) }
  }
  return <>
    <input ref={input} type="file" hidden multiple onChange={event => {
      const files = Array.from(event.target.files || []); event.target.value = ''; void upload(files)
    }} />
    <button type="button" onClick={() => input.current?.click()} disabled={busy || !web.canCompose}
      title="上传图片或文件到 ChatGPT"><Paperclip size={13} /><span>{busy ? '正在上传' : '附件'}</span></button>
    {error && <span role="alert">{error}</span>}
  </>
}

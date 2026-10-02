import { useEffect, useRef, useState } from 'react'
import { recordPath, recordRequest, type RecordCard, type RecordRow } from './recordApi'
import { getAuthToken } from '../../../api/client'
import { resolveApiUrl } from '../../../api/runtime'
import { useAuthStore } from '../../../store/auth'
import SocialImagePreview from '../SocialImagePreview'
import '../../../../../server/src/assets/chat_record_presentation.js'
import '../../../../../server/src/assets/chat_record_video.js'
import '../../../../../server/src/assets/chat_record_media.js'
import '../../../../../server/src/assets/chat_records.css'

export default function RecordAsset({ row, card }: { row: RecordRow; card: RecordCard }) {
  const host = useRef<HTMLDivElement>(null)
  const [preview, setPreview] = useState<{ url: string; name: string; trigger: HTMLElement } | null>(null)
  const owner = useAuthStore(s => s.user?.id)
  useEffect(() => () => { if (preview) URL.revokeObjectURL(preview.url) }, [preview])
  useEffect(() => {
    if (!host.current || !row.asset_id) return
    const controller = new AbortController(), token = getAuthToken()
    const path = `${recordPath(card)}/assets/${encodeURIComponent(row.asset_id)}`
    const dispose = ElonRecordMedia.mount(host.current, row, {
      current: () => !controller.signal.aborted && token === getAuthToken(),
      scope: `${resolveApiUrl(path)}:${owner}`,
      load: () => recordRequest(path, controller.signal, true) as Promise<Blob>,
      openImage: (blob, name, trigger) => {
        const url = URL.createObjectURL(blob)
        setPreview({ url, name, trigger })
        return () => setPreview(current => current?.url === url ? null : current)
      },
    })
    return () => { controller.abort(); dispose() }
  }, [row, card, owner])
  return <><div ref={host} className="chat-record-asset" />
    {preview && <SocialImagePreview url={preview.url} name={preview.name} returnFocus={preview.trigger} onClose={() => setPreview(null)} />}</>
}

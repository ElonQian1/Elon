import { useEffect, useRef } from 'react'
import { recordPath, recordRequest, type RecordCard, type RecordRow } from './recordApi'
import { getAuthToken } from '../../../api/client'
import { resolveApiUrl } from '../../../api/runtime'
import { useAuthStore } from '../../../store/auth'
import '../../../../../server/src/assets/chat_record_presentation.js'
import '../../../../../server/src/assets/chat_record_video.js'
import '../../../../../server/src/assets/chat_record_media.js'
import '../../../../../server/src/assets/chat_records.css'

export default function RecordAsset({ row, card }: { row: RecordRow; card: RecordCard }) {
  const host = useRef<HTMLDivElement>(null)
  const owner = useAuthStore(s => s.user?.id)
  useEffect(() => {
    if (!host.current || !row.asset_id) return
    const controller = new AbortController(), token = getAuthToken()
    const path = `${recordPath(card)}/assets/${encodeURIComponent(row.asset_id)}`
    const dispose = ElonRecordMedia.mount(host.current, row, {
      current: () => !controller.signal.aborted && token === getAuthToken(),
      scope: `${resolveApiUrl(path)}:${owner}`,
      load: () => recordRequest(path, controller.signal, true) as Promise<Blob>,
    })
    return () => { controller.abort(); dispose() }
  }, [row, card, owner])
  return <div ref={host} className="chat-record-asset" />
}

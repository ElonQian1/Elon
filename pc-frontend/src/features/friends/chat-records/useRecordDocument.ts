import { useEffect, useState } from 'react'
import { getAuthToken } from '../../../api/client'
import { useAuthStore } from '../../../store/auth'
import { RecordAccessError, recordPath, recordRequest, type RecordCard, type RecordView } from './recordApi'
import { clearRecordMedia } from './recordMediaLoader'
import { clearRecordDocument, documentLeaseRemaining, readRecordDocument, recordDocumentGeneration, saveRecordDocument, type CachedRecordDocument } from './recordDocumentCache'

export async function clearRecordContent(card: Pick<RecordCard, 'group_id' | 'record_id'>, owner: string) {
  await Promise.all([clearRecordDocument(card, owner), clearRecordMedia(card, owner)])
}

export function useRecordDocument(group_id: string, record_id: string, retry: number, token: string | null, owner = '') {
  const [view, setView] = useState<RecordView>(), [error, setError] = useState('')
  useEffect(() => {
    setView(undefined); setError('')
    const card = { group_id, record_id }, epoch = recordDocumentGeneration()
    let disposed = false, authoritative = false, local: CachedRecordDocument | undefined
    let request: AbortController | undefined
    let expiry: ReturnType<typeof setTimeout> | undefined, refresh: ReturnType<typeof setTimeout> | undefined
    const sameSession = () => epoch === recordDocumentGeneration() && token === getAuthToken()
      && owner === (useAuthStore.getState().user?.id || '')
    const current = () => !disposed && sameSession()
    const show = (entry: CachedRecordDocument) => {
      if (!current() || !documentLeaseRemaining(entry)) return
      local = entry
      // Revalidation must not remount media or move the user's reading position.
      setView(previous => previous && JSON.stringify(previous) === JSON.stringify(entry.view) ? previous : entry.view)
      clearTimeout(expiry); clearTimeout(refresh)
      // One bounded renewal while the reader stays open, before its lease expires.
      refresh = setTimeout(() => void validate(), Math.max(0, documentLeaseRemaining(entry) - 8000))
      expiry = setTimeout(() => {
        if (disposed) return
        setView(undefined); setError('缓存验证已过期，请重新连接后重试')
      }, documentLeaseRemaining(entry))
    }
    async function validate() {
      if (!current() || request) return
      const controller = new AbortController(); request = controller
      const timeout = setTimeout(() => controller.abort(), 8000)
      try {
        const value = await recordRequest(recordPath(card), controller.signal)
        if (!current()) return
        authoritative = true
        const entry = { view: value as RecordView, validatedAt: Date.now() }
        show(entry); setError('')
        void saveRecordDocument(card, owner, token || '', entry, sameSession)
      } catch (failure) {
        if (!current()) return
        if (failure instanceof RecordAccessError) {
          authoritative = true; clearTimeout(expiry); clearTimeout(refresh); setView(undefined)
          void clearRecordContent(card, owner)
        } else if (local && !documentLeaseRemaining(local)) { setView(undefined) }
        setError(controller.signal.aborted ? '读取超时，请重试' : failure instanceof Error ? failure.message : '读取失败，请重试')
      } finally { clearTimeout(timeout); request = undefined }
    }
    void readRecordDocument(card, owner, token || '').then(entry => { if (!authoritative && entry) show(entry) })
    void validate()
    return () => { disposed = true; request?.abort(); clearTimeout(expiry); clearTimeout(refresh) }
  }, [group_id, record_id, retry, token, owner])
  return { view, error, setView, setError }
}

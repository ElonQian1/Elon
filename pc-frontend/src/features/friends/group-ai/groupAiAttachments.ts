import { resolveApiUrl } from '../../../api/runtime'
import { getAuthToken } from '../../../api/client'
import type { PrivateUploadFile } from '../../user-browser/privateAttachmentUpload'

export interface GroupAiAttachment {
  message_id: string; attachment_id: string; name: string; mime_type: string
  size_bytes: number; download_path: string; sha256?: string | null
}

export function groupAttachmentFiles(manifest: GroupAiAttachment[]): PrivateUploadFile[] {
  return manifest.map(file => {
    const path = file.download_path
    const privateRecord = /^\/api\/me\/groups\/[\w-]+\/chat-records\/[\w-]+\/assets\/[\w-]+$/.test(path)
    if ((!privateRecord && !/^\/api\/user\/[^/]+\/chat-attachments\/[^/]+\/[^/?#]+$/.test(path))
      || /%2f|%5c|%00|%25/i.test(path) || path.includes('\\') || path.split('/').some(p => p === '.' || p === '..'))
      throw new Error('附件地址无效，未发送给 AI')
    const token = privateRecord ? getAuthToken() : null
    return { name: file.name, type: file.mime_type, size: file.size_bytes, sha256: file.sha256,
      async load() {
        const controller = new AbortController(), timer = setTimeout(() => controller.abort(), 45000)
        try {
          if (privateRecord && (!token || getAuthToken() !== token)) throw new Error('账号已变化，请重新选择记录')
          // Credentials go only to the exact protected platform record route, never to ChatGPT or redirects.
          const response = await fetch(resolveApiUrl(path), { signal: controller.signal, redirect: 'error', credentials: 'omit',
            cache: 'no-store', headers: token ? { Authorization: `Bearer ${token}` } : undefined })
          if (!response.ok || !response.body) throw new Error('无法读取所选附件，未发送给 AI')
          const reader = response.body.getReader(), parts: Uint8Array<ArrayBuffer>[] = []
          let length = 0
          try {
            for (;;) {
              const { done, value } = await reader.read()
              if (done) break
              length += value.length
              if (length > file.size_bytes || length > 8388608) throw new Error('附件大小已变化，未发送给 AI')
              parts.push(new Uint8Array(value))
            }
          } finally { await reader.cancel().catch(() => {}); reader.releaseLock() }
          if (length !== file.size_bytes) throw new Error('附件未完整下载，未发送给 AI')
          if (privateRecord && getAuthToken() !== token) throw new Error('账号已变化，请重新选择记录')
          return new Blob(parts, { type: file.mime_type })
        } finally { clearTimeout(timer) }
      },
    }
  })
}

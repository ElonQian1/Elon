import { useEffect, useRef, useState } from 'react'
import { ScanLine, ExternalLink, Copy, Download } from 'lucide-react'
import { openScanDialog } from '../../../../shared/scan/browserScanDialog.mjs'
import '../../../../shared/scan/browserScanDialog.css'
import { copyTextToClipboard } from '../../lib/clipboard'
import SocialContextMenu from '../friends/SocialContextMenu'
import { messageMenuRequest, copyImageAttachment, downloadAttachment, type SocialMenuRequest } from '../friends/socialMessageContext'
import SocialDialog from '../friends/SocialDialog'
import ScanFriendDialog from './ScanFriendDialog'
import { openImageScan } from './scanEntry'

export default function ImageScanHost() {
  const [menu, setMenu] = useState<{ request: SocialMenuRequest; url: string } | null>(null)
  const [friend, setFriend] = useState('')
  const [notice, setNotice] = useState('')
  const scanner = useRef<ReturnType<typeof openScanDialog>>()
  useEffect(() => {
    function open(event: Event) {
      scanner.current?.close()
      scanner.current = openScanDialog({
        imageUrl: (event as CustomEvent<{ imageUrl?: string }>).detail?.imageUrl,
        workerFactory: () => new Worker(new URL('../friends/source-links/qrWorker.ts', import.meta.url), { type: 'module' }),
        onFriend: setFriend,
      })
    }
    function context(event: MouseEvent) {
      const image = event.target
      // Chat attachments keep their full existing menu, with an added scan action.
      if (!(image instanceof HTMLImageElement) || image.closest('[data-social-attachment]')) return
      const url = image.currentSrc || image.src
      if (!url || !/^(https?:|blob:|data:image\/)/i.test(url)) return
      event.preventDefault(); event.stopPropagation()
      setMenu({ url, request: messageMenuRequest('scan-image', image, event.clientX, event.clientY, image) })
    }
    window.addEventListener('elon-open-scanner', open)
    document.addEventListener('contextmenu', context, true)
    return () => { window.removeEventListener('elon-open-scanner', open); document.removeEventListener('contextmenu', context, true); scanner.current?.close() }
  }, [])
  async function imageAction(action: () => Promise<unknown>, success: string) {
    try { await action(); setNotice(success) } catch { setNotice('图片操作失败，请打开原图后重试') }
  }
  return <>
    {menu && <SocialContextMenu request={menu.request} onClose={() => setMenu(null)} groups={[[
      { label: '识别图片二维码', icon: <ScanLine />, action: () => openImageScan(menu.url) },
      { label: '打开原图', icon: <ExternalLink />, action: () => { window.open(menu.url, '_blank', 'noopener,noreferrer') } },
      { label: '复制图片', icon: <Copy />, action: () => { void imageAction(() => copyImageAttachment({ url: menu.url }), '已复制图片') } },
      { label: '下载图片', icon: <Download />, action: () => { void imageAction(() => downloadAttachment({ url: menu.url }), '已交给下载管理器') } },
      { label: '复制图片地址', icon: <Copy />, action: () => { void imageAction(async () => { if (!await copyTextToClipboard(menu.url)) throw Error('copy failed') }, '已复制图片地址') } },
    ]]} />}
    {friend && <ScanFriendDialog id={friend} onClose={() => setFriend('')} />}
    {notice && <SocialDialog title="图片操作" onClose={() => setNotice('')}><p role="status">{notice}</p></SocialDialog>}
  </>
}

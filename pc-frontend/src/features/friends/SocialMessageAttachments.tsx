import { useEffect, useRef, useState } from 'react'
import { Download, FileText, X } from 'lucide-react'
import { cloudResourceUrl } from '../../lib/cloudResourceUrl'
import type { SocialAttachment } from './socialMessageTypes'
import styles from './SocialMessageAttachments.module.css'
import { attachmentKind } from './socialMessageContext'

export default function SocialMessageAttachments({ attachments }: { attachments?: SocialAttachment[] | null }) {
  return (
    <div className={styles.attachments}>
      {(attachments ?? []).map((attachment, index) => (
        <div key={`${attachment.attachment_id || attachment.url || index}:${index}`} data-social-attachment={index}>
          <Attachment attachment={attachment} />
        </div>
      ))}
    </div>
  )
}

function Attachment({ attachment }: { attachment: SocialAttachment }) {
  const url = cloudResourceUrl(attachment.url)
  const name = attachment.display_name || attachment.file_name || '附件'
  const kind = attachmentKind(attachment)
  if (!url) return <div className={styles.unavailable}>{name}：附件地址不可用</div>
  if (kind === 'image') return <ImageAttachment key={url} url={url} name={name} />
  if (kind === 'audio') return <AudioAttachment key={url} url={url} name={name} attachment={attachment} />
  return (
    <a className={styles.file} href={url} download>
      <FileText size={20} aria-hidden="true" /><span>{name}</span><Download size={16} aria-hidden="true" />
    </a>
  )
}

function ImageAttachment({ url, name }: { url: string; name: string }) {
  const [failed, setFailed] = useState(false)
  const [expanded, setExpanded] = useState(false)
  const [attempt, setAttempt] = useState(0)
  return (
    <>
      {failed ? (
        <div className={styles.unavailable} role="status">
          <span>图片加载失败：{name}</span>
          <button type="button" onClick={() => { setFailed(false); setAttempt(attempt + 1) }}>重新加载</button>
          <a href={url} download>下载图片</a>
        </div>
      ) : (
        <button type="button" className={styles.imageButton} data-preview-image aria-label={`查看图片：${name}`} onClick={() => setExpanded(true)}>
          <img key={attempt} className={styles.image} src={url} alt={name} loading="lazy" onError={() => setFailed(true)} />
        </button>
      )}
      {expanded && <ImagePreview url={url} name={name} onClose={() => setExpanded(false)} />}
    </>
  )
}

function ImagePreview({ url, name, onClose }: { url: string; name: string; onClose: () => void }) {
  const dialog = useRef<HTMLDialogElement>(null)
  useEffect(() => { dialog.current?.showModal() }, [])
  return (
    <dialog ref={dialog} className={styles.preview} aria-label={`图片预览：${name}`} onClose={onClose} onClick={event => {
      if (event.target === event.currentTarget) onClose()
    }}>
      <header><strong>{name}</strong><a href={url} download>下载原图</a>
        <button type="button" aria-label="关闭图片预览" autoFocus onClick={onClose}><X size={20} /></button>
      </header>
      <img src={url} alt={name} />
    </dialog>
  )
}

function AudioAttachment({ url, name, attachment }: { url: string; name: string; attachment: SocialAttachment }) {
  const [failed, setFailed] = useState(false)
  const audioRef = useRef<HTMLAudioElement>(null)
  const duration = attachment.duration_seconds
  useEffect(() => {
    const audio = audioRef.current
    return () => { audio?.pause() }
  }, [])
  return (
    <div className={styles.audio}>
      <strong>语音消息{duration && duration > 0 ? ` · ${duration} 秒` : ''}</strong>
      <audio ref={audioRef} controls preload="metadata" src={url} aria-label={`播放语音：${name}`}
        onError={() => setFailed(true)} onCanPlay={() => setFailed(false)}
        onPlay={event => {
          document.querySelectorAll('audio').forEach(audio => { if (audio !== event.currentTarget) audio.pause() })
        }} />
      {failed && <div className={styles.unavailable} role="status">
        <span>语音暂时无法播放</span>
        <button type="button" onClick={() => { setFailed(false); audioRef.current?.load() }}>重试</button>
        <a href={url} download>下载语音</a>
      </div>}
      {attachment.transcription && <p>{attachment.transcription}</p>}
    </div>
  )
}

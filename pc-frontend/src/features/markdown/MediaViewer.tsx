import { useEffect, useState } from 'react'
import { createPortal } from 'react-dom'
import { ChevronLeft, ChevronRight, ExternalLink, X } from 'lucide-react'
import styles from './MediaViewer.module.css'

export interface MediaViewerImage {
  src: string
  alt: string
}

interface MediaViewerProps {
  images: MediaViewerImage[]
  index: number
  onClose: () => void
}

export function MediaViewer({ images, index, onClose }: MediaViewerProps) {
  const imageCount = images.length
  const [currentIndex, setCurrentIndex] = useState(() => clampIndex(index, imageCount))

  useEffect(() => {
    setCurrentIndex(clampIndex(index, imageCount))
  }, [index, imageCount])

  const currentImage = images[currentIndex]
  const hasMultiple = imageCount > 1

  useEffect(() => {
    if (!currentImage) return undefined
    const previousOverflow = document.body.style.overflow
    document.body.style.overflow = 'hidden'

    const handleKeyDown = (event: KeyboardEvent) => {
      if (event.key === 'Escape') {
        event.preventDefault()
        onClose()
      }
      if (event.key === 'ArrowLeft' && imageCount > 1) {
        event.preventDefault()
        setCurrentIndex((value) => (value + imageCount - 1) % imageCount)
      }
      if (event.key === 'ArrowRight' && imageCount > 1) {
        event.preventDefault()
        setCurrentIndex((value) => (value + 1) % imageCount)
      }
    }

    window.addEventListener('keydown', handleKeyDown)
    return () => {
      document.body.style.overflow = previousOverflow
      window.removeEventListener('keydown', handleKeyDown)
    }
  }, [currentImage, imageCount, onClose])

  if (!currentImage) return null

  const showPrevious = () => setCurrentIndex((value) => (value + imageCount - 1) % imageCount)
  const showNext = () => setCurrentIndex((value) => (value + 1) % imageCount)
  const dialogLabel = currentImage.alt || '图片预览'

  return createPortal(
    <div
      className={styles.backdrop}
      role="dialog"
      aria-modal="true"
      aria-label={dialogLabel}
      onClick={onClose}
    >
      <div className={styles.toolbar} onClick={(event) => event.stopPropagation()}>
        {hasMultiple && (
          <span className={styles.counter}>
            {currentIndex + 1}/{imageCount}
          </span>
        )}
        <a
          className={styles.iconBtn}
          href={currentImage.src}
          target="_blank"
          rel="noopener noreferrer"
          title="新窗口打开"
          aria-label="新窗口打开"
        >
          <ExternalLink size={18} aria-hidden="true" />
        </a>
        <button className={styles.iconBtn} type="button" onClick={onClose} title="关闭" aria-label="关闭">
          <X size={20} aria-hidden="true" />
        </button>
      </div>

      {hasMultiple && (
        <button
          className={`${styles.navBtn} ${styles.prevBtn}`}
          type="button"
          onClick={(event) => {
            event.stopPropagation()
            showPrevious()
          }}
          title="上一张"
          aria-label="上一张"
        >
          <ChevronLeft size={26} aria-hidden="true" />
        </button>
      )}

      <img
        src={currentImage.src}
        alt={currentImage.alt}
        className={styles.image}
        onClick={(event) => event.stopPropagation()}
        onContextMenu={(event) => event.stopPropagation()}
      />

      {hasMultiple && (
        <button
          className={`${styles.navBtn} ${styles.nextBtn}`}
          type="button"
          onClick={(event) => {
            event.stopPropagation()
            showNext()
          }}
          title="下一张"
          aria-label="下一张"
        >
          <ChevronRight size={26} aria-hidden="true" />
        </button>
      )}

      {currentImage.alt && (
        <div className={styles.caption} onClick={(event) => event.stopPropagation()}>
          {currentImage.alt}
        </div>
      )}
    </div>,
    document.body,
  )
}

function clampIndex(index: number, imageCount: number) {
  if (imageCount <= 0) return 0
  return Math.min(Math.max(index, 0), imageCount - 1)
}

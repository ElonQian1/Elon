import { type UIEvent, useCallback, useLayoutEffect, useRef, useState } from 'react'
import type { Message } from './types'

const FEED_BOTTOM_THRESHOLD_PX = 80

interface FeedScrollSnapshot {
  scrollTop: number
  scrollHeight: number
  clientHeight: number
}

interface ConversationAutoScrollOptions {
  messages: Message[]
  convMessages: Message[]
  sessionTaskMessages: Message[]
  sessionView: string | 'new' | null
  sendingMessage: boolean
  sendingMemberDiscussion: boolean
}

function isFeedNearBottom(el: HTMLDivElement) {
  return el.scrollHeight - el.scrollTop - el.clientHeight < FEED_BOTTOM_THRESHOLD_PX
}

function wasFeedNearBottom(snapshot: FeedScrollSnapshot | null) {
  if (!snapshot) return false
  return snapshot.scrollHeight - snapshot.scrollTop - snapshot.clientHeight < FEED_BOTTOM_THRESHOLD_PX
}

export function useConversationAutoScroll({
  messages,
  convMessages,
  sessionTaskMessages,
  sessionView,
  sendingMessage,
  sendingMemberDiscussion,
}: ConversationAutoScrollOptions) {
  const [showNewMsg, setShowNewMsg] = useState(false)
  const feedRef = useRef<HTMLDivElement>(null)
  const feedShouldFollowRef = useRef(true)
  const forceNextFeedFollowRef = useRef(false)
  const feedScrollSnapshotRef = useRef<FeedScrollSnapshot | null>(null)

  const captureFeedScroll = useCallback((el: HTMLDivElement) => {
    const atBottom = isFeedNearBottom(el)
    feedShouldFollowRef.current = atBottom
    feedScrollSnapshotRef.current = {
      scrollTop: el.scrollTop,
      scrollHeight: el.scrollHeight,
      clientHeight: el.clientHeight,
    }
    return atBottom
  }, [])

  const requestFeedAutoFollow = useCallback(() => {
    feedShouldFollowRef.current = true
    forceNextFeedFollowRef.current = true
    const el = feedRef.current
    if (el) {
      el.scrollTop = el.scrollHeight
      captureFeedScroll(el)
    }
    setShowNewMsg(false)
  }, [captureFeedScroll])

  useLayoutEffect(() => {
    const el = feedRef.current
    if (!el) return
    const snapshot = feedScrollSnapshotRef.current
    const shouldFollow = forceNextFeedFollowRef.current
      || feedShouldFollowRef.current
      || (!snapshot && isFeedNearBottom(el))
      || wasFeedNearBottom(snapshot)
    forceNextFeedFollowRef.current = false
    if (shouldFollow) {
      el.scrollTop = el.scrollHeight
      captureFeedScroll(el)
      setShowNewMsg(false)
      return
    }

    if (snapshot) {
      const maxScrollTop = Math.max(0, el.scrollHeight - el.clientHeight)
      el.scrollTop = Math.min(snapshot.scrollTop, maxScrollTop)
    }
    captureFeedScroll(el)
    feedShouldFollowRef.current = false
    setShowNewMsg(true)
  }, [messages, convMessages, sessionTaskMessages, sessionView, sendingMessage, sendingMemberDiscussion, captureFeedScroll])

  const handleFeedScroll = useCallback((event?: UIEvent<HTMLDivElement>) => {
    const el = event?.currentTarget ?? feedRef.current
    if (!el) return
    if (captureFeedScroll(el)) {
      setShowNewMsg(false)
    }
  }, [captureFeedScroll])

  const scrollToBottom = useCallback(() => {
    const el = feedRef.current
    if (!el) return
    el.scrollTop = el.scrollHeight
    feedShouldFollowRef.current = true
    forceNextFeedFollowRef.current = false
    captureFeedScroll(el)
    setShowNewMsg(false)
  }, [captureFeedScroll])

  return {
    feedRef,
    handleFeedScroll,
    requestFeedAutoFollow,
    scrollToBottom,
    showNewMsg,
  }
}

import { useEffect, useState } from 'react'
import { isLocalWorkbench } from '../../api/runtime'

const CHANNEL_NAME = 'elon-pc-workbench-single-tab'

type TabMessage =
  | { type: 'hello'; from: string; openedAt: number }
  | { type: 'present'; from: string; to: string; openedAt: number }

export function useWorkbenchTabCoordinator(): boolean {
  const [duplicate, setDuplicate] = useState(false)

  useEffect(() => {
    if (!('BroadcastChannel' in window)) return
    if (!shouldCoordinateWorkbenchTabs()) return

    const openedAt = Date.now()
    const tabId = `${openedAt}-${Math.random().toString(36).slice(2)}`
    const channel = new BroadcastChannel(CHANNEL_NAME)
    let noticeTimer: number | undefined

    function retireDuplicateTab() {
      if (noticeTimer) return
      try {
        window.close()
      } catch {
        // Some browsers block closing tabs not opened by script.
      }
      noticeTimer = window.setTimeout(() => setDuplicate(true), 240)
    }

    channel.onmessage = (event: MessageEvent<TabMessage>) => {
      const message = event.data
      if (!message || message.from === tabId) return

      if (message.type === 'hello') {
        if (!isOlderTab(openedAt, tabId, message.openedAt, message.from)) return
        channel.postMessage({ type: 'present', from: tabId, to: message.from, openedAt } satisfies TabMessage)
        try {
          window.focus()
        } catch {
          // Best effort only.
        }
        return
      }

      if (message.type === 'present' && message.to === tabId) {
        retireDuplicateTab()
      }
    }

    channel.postMessage({ type: 'hello', from: tabId, openedAt } satisfies TabMessage)

    return () => {
      if (noticeTimer) window.clearTimeout(noticeTimer)
      channel.close()
    }
  }, [])

  return duplicate
}

function shouldCoordinateWorkbenchTabs(): boolean {
  return isLocalWorkbench() || new URLSearchParams(location.search).has('node_admin')
}

function isOlderTab(openedAt: number, tabId: string, otherOpenedAt: number, otherTabId: string): boolean {
  return openedAt < otherOpenedAt || (openedAt === otherOpenedAt && tabId < otherTabId)
}

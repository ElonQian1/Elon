import { useState } from 'react'
import { isLocalAiBrowserAvailable } from '../user-browser/localAiBrowserApi'
import AiChatPage from './AiChatPage'
import type { AiHomeMode } from './AiHomeModeSwitch'

const MODE_STORAGE_KEY = 'elon.pc.aiHomeMode'

export default function AiHomePage() {
  const [mode, setMode] = useState<AiHomeMode>(readInitialMode)

  function changeMode(nextMode: AiHomeMode) {
    setMode(nextMode)
    try { window.localStorage.setItem(MODE_STORAGE_KEY, nextMode) } catch {}
  }

  return <AiChatPage mode={mode} onModeChange={changeMode} />
}

function readInitialMode(): AiHomeMode {
  const desktopChatAvailable = isLocalAiBrowserAvailable()
  try {
    const stored = window.localStorage.getItem(MODE_STORAGE_KEY)
    if (stored === 'work') return 'work'
    if (stored === 'chat' && desktopChatAvailable) return 'chat'
  } catch {}
  return desktopChatAvailable ? 'chat' : 'work'
}

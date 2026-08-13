import { useState } from 'react'
import AuthDialog from '../auth/AuthDialog'
import UnifiedWebChat from '../user-browser/UnifiedWebChat'
import { isLocalAiBrowserAvailable } from '../user-browser/localAiBrowserApi'
import AiChatPage from './AiChatPage'
import AiHomeModeSwitch, { type AiHomeMode } from './AiHomeModeSwitch'
import styles from './AiHomePage.module.css'

const MODE_STORAGE_KEY = 'elon.pc.aiHomeMode'

export default function AiHomePage() {
  const [mode, setMode] = useState<AiHomeMode>(readInitialMode)
  const [loginDialogOpen, setLoginDialogOpen] = useState(false)

  function changeMode(nextMode: AiHomeMode) {
    setMode(nextMode)
    try { window.localStorage.setItem(MODE_STORAGE_KEY, nextMode) } catch {}
  }

  return (
    <>
      {mode === 'chat' ? (
        <UnifiedWebChat
          mode={mode}
          onModeChange={changeMode}
          onLogin={() => setLoginDialogOpen(true)}
        />
      ) : (
        <div className={styles.workMode}>
          <AiChatPage />
          <div className={styles.workModeSwitch}>
            <AiHomeModeSwitch mode={mode} onChange={changeMode} />
          </div>
        </div>
      )}
      <AuthDialog
        open={loginDialogOpen}
        initialMode="login"
        onClose={() => setLoginDialogOpen(false)}
      />
    </>
  )
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

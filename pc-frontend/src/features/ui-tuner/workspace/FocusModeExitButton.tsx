import styles from '../UiTunerPage.module.css'

interface FocusModeExitButtonProps {
  active: boolean
  onExit: () => void
}

export function FocusModeExitButton({ active, onExit }: FocusModeExitButtonProps) {
  if (!active) return null

  return (
    <button
      type="button"
      className={styles.focusModeExit}
      onClick={onExit}
      aria-label="退出专注画布"
      title="退出专注画布（Esc）"
    >
      退出专注
      <kbd>Esc</kbd>
    </button>
  )
}

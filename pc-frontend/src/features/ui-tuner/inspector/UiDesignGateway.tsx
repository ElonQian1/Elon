import styles from './UiDesignGateway.module.css'

interface UiDesignGatewayProps {
  runtimeReady: boolean
  runtimeBusy: boolean
  runtimeError?: string
  onUseDraftNow: () => void
}

export function UiDesignGateway({
  runtimeReady,
  runtimeBusy,
  runtimeError,
  onUseDraftNow,
}: UiDesignGatewayProps) {
  const preparingRuntime = runtimeReady || runtimeBusy
  const hasRuntimeError = Boolean(runtimeError)
  return (
    <section className={styles.gateway} data-testid="automatic-design-setup">
      <div className={styles.heading}>
        <span className={styles.spinner} aria-hidden="true" />
        <div>
          <span className={styles.eyebrow}>正在自动准备 · 无需选择模式</span>
          <strong>
            {hasRuntimeError
              ? '真实 Runtime 暂不可用，正在切到 PWA 草稿'
              : preparingRuntime
                ? '正在连接真实 Android 编辑环境'
                : '正在建立本地可编辑草稿'}
          </strong>
        </div>
      </div>
      <p>
        {hasRuntimeError
          ? '你仍然可以先在 PC 端真实 PWA 页面上修改样式；Android 真机稍后作为校准和写回验收，不会阻塞设计。'
          : preparingRuntime
          ? '系统会自动连接当前手机；如果连接较慢，会先打开本地草稿，真实 Android 在后台继续准备。'
          : '系统正在读取项目源码；准备完成后会直接进入可编辑画布。'}
      </p>
      <ol className={styles.steps}>
        <li className={styles.done}>已识别当前 Android 组件</li>
        <li className={styles.active}>自动选择最准确的编辑引擎</li>
        <li>连接较慢时先进入草稿，不阻塞设计</li>
      </ol>
      <button type="button" className={styles.primaryAction} onClick={onUseDraftNow}>
        先进入 PWA / 本地草稿设计
      </button>
      <small>你不需要等待 Runtime；后台准备完成后，系统仍会用 Android 真帧校准写回结果。</small>
    </section>
  )
}

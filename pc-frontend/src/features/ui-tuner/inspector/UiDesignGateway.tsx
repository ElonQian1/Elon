import styles from './UiDesignGateway.module.css'

interface UiDesignGatewayProps {
  runtimeReady: boolean
  runtimeBusy: boolean
}

export function UiDesignGateway({
  runtimeReady,
  runtimeBusy,
}: UiDesignGatewayProps) {
  const preparingRuntime = runtimeReady || runtimeBusy
  return (
    <section className={styles.gateway} data-testid="automatic-design-setup">
      <div className={styles.heading}>
        <span className={styles.spinner} aria-hidden="true" />
        <div>
          <span className={styles.eyebrow}>正在自动准备 · 无需选择模式</span>
          <strong>{preparingRuntime ? '正在连接真实 Android 编辑环境' : '正在建立本地可编辑草稿'}</strong>
        </div>
      </div>
      <p>
        {preparingRuntime
          ? '系统会自动构建调试环境、连接当前手机，并把右侧切换成可直接修改的样式面板。'
          : '系统正在读取项目源码；准备完成后会直接进入可编辑画布。'}
      </p>
      <ol className={styles.steps}>
        <li className={styles.done}>已识别当前 Android 组件</li>
        <li className={styles.active}>自动选择最准确的编辑引擎</li>
        <li>准备完成后直接显示样式属性</li>
      </ol>
      <small>你不需要配置 Runtime、草稿或源码绑定；系统会自动选择并在写回后用 Android 真帧校准。</small>
    </section>
  )
}

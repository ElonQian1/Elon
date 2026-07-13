import { Download, FileDown, Power, RefreshCw, Wrench } from 'lucide-react'
import { WIN_CLIENT_DOWNLOAD_URL } from './launchWinClient'
import styles from './LocalNodeReliability.module.css'

interface Props {
  onLaunch: () => void
  onRetry: () => void
}

export default function LocalNodeOfflineCard({ onLaunch, onRetry }: Props) {
  return (
    <section className={styles.panel}>
      <div className={styles.offlineLayout}>
        <div className={styles.header}>
          <div>
            <p className={styles.eyebrow}>本机程序未响应</p>
            <h3 className={styles.title}>Win 端未启动，或本机管理接口卡住</h3>
            <p className={styles.detail}>
              网页现在无法访问本机节点端口。请先拉起 Win 端；已安装但仍不可用时，直接修复入口或导出诊断。
            </p>
          </div>
          <span className={[styles.badge, styles.toneDanger].join(' ')}>未连接</span>
        </div>

        <div className={styles.actions}>
          <a className={[styles.button, styles.primary].join(' ')} href={WIN_CLIENT_DOWNLOAD_URL} download>
            <Download size={16} aria-hidden="true" />
            下载 Win 端
          </a>
          <button className={styles.button} onClick={onLaunch}>
            <Power size={16} aria-hidden="true" />
            启动 Win 端
          </button>
          <button className={styles.button} onClick={onRetry}>
            <RefreshCw size={16} aria-hidden="true" />
            重新检测
          </button>
          <a className={styles.button} href="elon-node://repair">
            <Wrench size={16} aria-hidden="true" />
            修复客户端
          </a>
          <a className={styles.button} href="elon-node://diagnostics/export">
            <FileDown size={16} aria-hidden="true" />
            导出诊断
          </a>
        </div>

        <div className={styles.stepList}>
          <div className={styles.step}><strong>1</strong><span>如果还没安装，下载压缩包并双击「一龙开发平台.exe」。</span></div>
          <div className={styles.step}><strong>2</strong><span>如果已安装，点击“启动 Win 端”；后台守护层会继续拉起或重启节点。</span></div>
          <div className={styles.step}><strong>3</strong><span>如果多次失败，点击“导出诊断”，把生成的脱敏文件发给客服或开发者。</span></div>
        </div>

        <div className={styles.securityNote}>
          <strong>安全软件提示</strong>
          <span>如果 Windows 安全中心隔离了「一龙开发平台.exe」，请只还原这一个官方文件，再重新检测；不要把整个 ElonNode 目录加入白名单。</span>
        </div>
      </div>
    </section>
  )
}

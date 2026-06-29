import styles from './NodePage.module.css'

export type RouteConfigKey = 'route_a' | 'route_b' | 'route_c2' | 'route_c3'

interface RouteConfigGuide {
  code: string
  title: string
  body: string
  checks: string[]
  target: string
}

const ROUTE_CONFIG_GUIDES: Record<RouteConfigKey, RouteConfigGuide> = {
  route_a: {
    code: 'Route A',
    title: '配置本机 CLI',
    body: '这条路线要求项目绑定的 PC 节点已安装并登录 Codex、Copilot、Claude 或 Gemini CLI。',
    checks: [
      '本机 Win 端已启动并绑定当前账号',
      '节点详情里的 AI Agent / CLI Runtime 显示就绪',
      '目标 CLI 已在该电脑上完成登录授权',
    ],
    target: '本机节点或项目绑定的个人 PC 节点',
  },
  route_b: {
    code: 'Route B',
    title: '配置自带 API key',
    body: '这条路线要求本机 PC 节点保存 OpenAI-compatible API key、base URL 和模型名。',
    checks: [
      '本机 Win 端已启动并绑定当前账号',
      '在高级本机页保存 API Runtime 配置',
      '节点详情里的 API Runtime 显示就绪',
    ],
    target: '本机 PC harness',
  },
  route_c2: {
    code: 'Route C.2',
    title: '配置远程 PC API Runtime',
    body: '这条路线要求目标远程 PC 节点可接项目，并且该节点的 API Runtime 已配置。',
    checks: [
      '节点在线且有项目容量',
      '节点工作区、Git 和开发环境就绪',
      '节点详情里的 API Runtime 显示就绪',
    ],
    target: '服务器节点大厅中的远程 PC 节点',
  },
  route_c3: {
    code: 'Route C.3',
    title: '配置远程 PC CLI',
    body: '这条路线要求目标远程 PC 节点可接项目，并且该节点已登录可用的 Codex / Copilot CLI。',
    checks: [
      '节点在线且有项目容量',
      '节点工作区、Git 和开发环境就绪',
      '节点详情里的 AI Agent / CLI Runtime 显示就绪',
    ],
    target: '服务器节点大厅中的远程 PC 节点',
  },
}

export function isRouteConfigKey(value: unknown): value is RouteConfigKey {
  return typeof value === 'string'
    && Object.prototype.hasOwnProperty.call(ROUTE_CONFIG_GUIDES, value)
}

export default function RuntimeRouteConfigGuide({ route }: { route: RouteConfigKey }) {
  const guide = ROUTE_CONFIG_GUIDES[route]
  return (
    <section className={styles.routeGuide} aria-label={`${guide.code} 配置`}>
      <div className={styles.routeGuideHead}>
        <span className={styles.routeGuideCode}>{guide.code}</span>
        <div>
          <h3>{guide.title}</h3>
          <p>{guide.body}</p>
        </div>
      </div>
      <div className={styles.routeGuideGrid}>
        <div>
          <strong>配置对象</strong>
          <span>{guide.target}</span>
        </div>
        <div>
          <strong>就绪检查</strong>
          <ul>
            {guide.checks.map((item) => <li key={item}>{item}</li>)}
          </ul>
        </div>
      </div>
    </section>
  )
}

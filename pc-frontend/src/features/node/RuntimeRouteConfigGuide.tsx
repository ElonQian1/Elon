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
    code: '本机AI',
    title: '配置本机AI',
    body: '这种方式会使用项目电脑上已经登录的 Codex、Copilot 或 Claude。',
    checks: [
      '本机 Win 端已启动并绑定当前账号',
      '节点详情里的本机AI显示就绪',
      '目标 AI 工具已在该电脑上完成登录授权',
    ],
    target: '本机节点或项目绑定的个人 PC 节点',
  },
  route_b: {
    code: '我的Key',
    title: '配置本机 API key',
    body: '这种方式会使用你自己的模型 key，并由一龙在本机完成项目开发动作。',
    checks: [
      '本机 Win 端已启动并绑定当前账号',
      '在高级本机页保存 API key、模型地址和模型名',
      '节点详情里的本机 API key 显示就绪',
    ],
    target: '本机一龙开发环境',
  },
  route_c2: {
    code: '远程AI',
    title: '配置其他用户 PC 节点 + 一龙 CLI',
    body: '这种方式会把项目交给其他在线电脑执行，并使用那台电脑准备好的一龙 AI 能力。',
    checks: [
      '节点在线且有项目容量',
      '节点工作区、Git 和开发环境就绪',
      '节点详情里的本机 API key 或一龙 AI 能力显示就绪',
    ],
    target: '服务器节点大厅中的其他用户 PC 节点',
  },
  route_c3: {
    code: '远程Codex',
    title: '配置其他用户 PC 节点 + Codex / Claude',
    body: '这种方式会使用其他在线电脑上已经登录的 Codex、Claude 或 Copilot 来执行项目。',
    checks: [
      '节点在线且有项目容量',
      '节点工作区、Git 和开发环境就绪',
      '节点详情里的本机AI显示就绪',
    ],
    target: '服务器节点大厅中的其他用户 PC 节点',
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

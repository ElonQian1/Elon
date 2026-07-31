import { useState } from 'react'
import {
  Blocks,
  Bot,
  Coins,
  Compass,
  DatabaseZap,
  Store,
} from 'lucide-react'
import AiResourceControlPanel from './AiResourceControlPanel'
import ConsumerCommerceSandbox from './ConsumerCommerceSandbox'
import DeveloperCommercePortal from './DeveloperCommercePortal'
import OpenCommerceMerchantWorkspace from './OpenCommerceMerchantWorkspace'
import ShadowEconomyPanel from './ShadowEconomyPanel'
import ErpBlueprintPanel from './erp-blueprint/ErpBlueprintPanel'
import base from './OpenCommercePanel.module.css'
import { commerceStyles, tabStyle } from './openCommerceStyles'

type WorkspaceView = 'merchant' | 'erp' | 'consumer' | 'developer' | 'resources' | 'economy'

const views: Array<{
  id: WorkspaceView
  label: string
  icon: typeof Store
}> = [
  { id: 'merchant', label: '商户节点', icon: Store },
  { id: 'erp', label: 'ERP 蓝图', icon: DatabaseZap },
  { id: 'consumer', label: '消费者沙盒', icon: Compass },
  { id: 'developer', label: '开发者', icon: Blocks },
  { id: 'resources', label: 'AI 资源', icon: Bot },
  { id: 'economy', label: '影子经济', icon: Coins },
]

export default function OpenCommercePanel({
  projectId,
  canEdit,
}: {
  projectId: string
  canEdit: boolean
}) {
  const [view, setView] = useState<WorkspaceView>('merchant')

  return (
    <div className={base.panel}>
      <nav style={commerceStyles.tabs} aria-label="开放商业工作区">
        {views.map(({ id, label, icon: Icon }) => (
          <button
            key={id}
            type="button"
            data-active={view === id}
            style={tabStyle(view === id)}
            onClick={() => setView(id)}
          >
            <Icon size={15} aria-hidden="true" />
            <span>{label}</span>
          </button>
        ))}
      </nav>

      {view === 'merchant' && (
        <OpenCommerceMerchantWorkspace projectId={projectId} canEdit={canEdit} />
      )}
      {view === 'erp' && <ErpBlueprintPanel projectId={projectId} canEdit={canEdit} />}
      {view === 'consumer' && <ConsumerCommerceSandbox projectId={projectId} />}
      {view === 'developer' && (
        <DeveloperCommercePortal projectId={projectId} canEdit={canEdit} />
      )}
      {view === 'resources' && (
        <AiResourceControlPanel projectId={projectId} canEdit={canEdit} />
      )}
      {view === 'economy' && (
        <ShadowEconomyPanel projectId={projectId} canEdit={canEdit} />
      )}
    </div>
  )
}

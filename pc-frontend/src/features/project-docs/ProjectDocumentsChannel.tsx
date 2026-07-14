import { v4 as uuidv4 } from 'uuid'

import { safeNodeAdminUrl } from '../../lib/utils'
import { channelAllowsAiStart } from '../conversation/conversationPageHelpers'
import { ensureLocalFullAccessGrant } from '../conversation/localPcRuntime'
import type { RuntimeRoute } from '../conversation/runtimeRoutes'
import type { Channel } from '../conversation/types'
import { useProjectStore } from '../conversation/useProjectStore'
import { selectedAgentForRuntimeRoute } from '../models/routeModelPolicy'
import type { AgentOption } from '../models/types'
import ProjectDocumentsWorkspace from './ProjectDocumentsWorkspace'

export interface ProjectDocumentsRuntime {
  projectId: string
  projectName: string
  activeWorkspacePath: string
  runtimePermission?: string
  channels: Channel[]
  aiDevelopmentChannel?: Channel
  directPcCliActive: boolean
  runtimeRoute: RuntimeRoute
  shouldPreferLocalNode: boolean
  localNodeReady: boolean
  localNodeId: string
  selectedAgent: string
  modelOptions: AgentOption[]
}

interface Props {
  runtime: ProjectDocumentsRuntime
  onOpenProjectHome: () => void | Promise<void>
}

export default function ProjectDocumentsChannel({ runtime, onOpenProjectHome }: Props) {
  const aiChannel = runtime.aiDevelopmentChannel
  const canStartAi = !!aiChannel && channelAllowsAiStart(aiChannel)
  const returnChannel = aiChannel ?? runtime.channels.find((channel) => channel.kind !== 'docs')

  async function startOrganization(prompt: string) {
    if (!aiChannel || !canStartAi) throw new Error('当前项目没有可用的 AI 开发频道')
    const requestRoute: RuntimeRoute = runtime.directPcCliActive ? 'route_a' : runtime.runtimeRoute
    const useLocalNode = (runtime.directPcCliActive || runtime.shouldPreferLocalNode) && runtime.localNodeReady
    const requestAgent = selectedAgentForRuntimeRoute(runtime.selectedAgent, runtime.modelOptions, requestRoute)
    await ensureLocalFullAccessGrant({
      adminUrl: safeNodeAdminUrl(),
      projectId: runtime.projectId,
      projectName: runtime.projectName,
      workspacePath: runtime.activeWorkspacePath,
      runtimePermission: runtime.runtimePermission,
      useLocalRouteA: useLocalNode && requestRoute === 'route_a',
    })
    const projectStore = useProjectStore.getState()
    await projectStore.sendMessage(
      prompt,
      requestAgent || null,
      requestRoute,
      uuidv4(),
      '项目文档低 token 整理实验',
      useLocalNode ? runtime.localNodeId : null,
      useLocalNode ? runtime.activeWorkspacePath : null,
      aiChannel.id,
      runtime.directPcCliActive,
    )
    await projectStore.selectChannel(aiChannel.id)
  }

  return (
    <ProjectDocumentsWorkspace
      projectId={runtime.projectId}
      projectName={runtime.projectName}
      canStartAi={canStartAi}
      onBack={() => {
        if (returnChannel) useProjectStore.getState().selectChannel(returnChannel.id)
        else onOpenProjectHome()
      }}
      onStartAiOrganize={startOrganization}
    />
  )
}

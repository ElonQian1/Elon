import { lazy } from 'react'

export const AiChatMessageRow = lazy(() => import('./AiChatMessageRow'))
export const AiWebChatSidebar = lazy(() => import('../user-browser/AiWebChatSidebar'))
export const AiWebProviderPopover = lazy(() => import('../user-browser/AiWebProviderPopover'))
export const AiWebComposerControls = lazy(() => import('../user-browser/AiWebComposerControls'))
export const AiWebClientUpgradeNotice = lazy(() => import('./AiWebClientUpgradeNotice'))
export const AiUserProfilePopover = lazy(() => import('./AiUserProfilePopover'))
export const NodeStatusBanner = lazy(() => import('./NodeStatusBanner'))
export const AuthDialog = lazy(() => import('../auth/AuthDialog'))
export const ModelPickerPopover = lazy(async () => {
  const module = await import('../models/ModelPicker')
  return { default: module.ModelPickerPopover }
})

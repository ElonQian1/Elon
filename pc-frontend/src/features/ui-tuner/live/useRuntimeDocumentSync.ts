import { useEffect, useRef, type Dispatch, type MutableRefObject, type SetStateAction } from 'react'
import type { SourcePreviewMode } from '../source-preview/types'
import type { UiTunerDocument, UiTunerElement } from '../types'
import { preferredRuntimeSelection, runtimeNodesToTunerDocument } from './runtimeNodeDocument'
import { useLiveUiSession } from './useLiveUiSession'
import type { AndroidDeviceLeaseProof } from '../device/deviceLeaseApi'

interface RuntimeDocumentSyncOptions {
  deviceId?: string
  packageName?: string
  projectRoot?: string
  debugApplicationIdSuffix?: string
  lease?: AndroidDeviceLeaseProof
  document: UiTunerDocument
  selected: UiTunerElement | null
  workspaceMode: SourcePreviewMode
  documentRef: MutableRefObject<UiTunerDocument>
  selectedIdRef: MutableRefObject<string | null>
  setDocument: Dispatch<SetStateAction<UiTunerDocument>>
  setSelectedId: Dispatch<SetStateAction<string | null>>
  onNotice: (message: string) => void
}

export function useRuntimeDocumentSync(options: RuntimeDocumentSyncOptions) {
  const signatureRef = useRef('')
  const liveUi = useLiveUiSession({
    deviceId: options.deviceId,
    packageName: options.packageName,
    projectRoot: options.projectRoot,
    debugApplicationIdSuffix: options.debugApplicationIdSuffix,
    lease: options.lease,
    document: options.document,
    selected: options.selected,
    onNotice: options.onNotice,
  })

  useEffect(() => {
    if (
      options.workspaceMode !== 'evidence'
      || liveUi.state !== 'connected'
      || !liveUi.session
      || !liveUi.liveFrame
      || liveUi.nodes.length === 0
    ) {
      if (liveUi.state !== 'connected') signatureRef.current = ''
      return
    }
    const signature = [
      liveUi.session.id,
      liveUi.session.treeRevision,
      liveUi.nodes.length,
      liveUi.liveFrame.width,
      liveUi.liveFrame.height,
    ].join(':')
    if (signatureRef.current === signature) return
    signatureRef.current = signature
    const previousSelected = options.documentRef.current.elements.find(
      (element) => element.id === options.selectedIdRef.current,
    ) ?? null
    options.setDocument((current) => {
      const next = runtimeNodesToTunerDocument(
        current,
        liveUi.session!,
        liveUi.nodes,
        liveUi.liveFrame!,
      )
      options.setSelectedId(preferredRuntimeSelection(previousSelected, next.elements))
      return next
    })
  }, [
    liveUi.liveFrame,
    liveUi.nodes,
    liveUi.session,
    liveUi.state,
    options,
  ])

  return liveUi
}

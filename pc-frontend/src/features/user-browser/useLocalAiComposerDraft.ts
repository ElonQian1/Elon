import { useCallback, useEffect, useRef, useState } from 'react'
import {
  localAiProviderDraftCache,
  localAiProviderDraftIdentity,
} from './localAiProviderDraftCache'

export default function useLocalAiComposerDraft(providerId: string, ownerKey: string) {
  const identity = localAiProviderDraftIdentity(providerId, ownerKey)
  const initialDraft = localAiProviderDraftCache.read(identity)
  const [draft, setDraftValue] = useState(initialDraft)
  const [draftTouched, setDraftTouched] = useState(Boolean(initialDraft))
  const draftRef = useRef(initialDraft)
  const activeIdentity = useRef(identity)

  useEffect(() => {
    if (activeIdentity.current === identity) return
    localAiProviderDraftCache.remember(activeIdentity.current, draftRef.current)
    const next = ownerKey
      ? localAiProviderDraftCache.claimPending(providerId, ownerKey)
      : localAiProviderDraftCache.read(identity)
    activeIdentity.current = identity
    draftRef.current = next
    setDraftValue(next)
    setDraftTouched(Boolean(next))
  }, [identity, ownerKey, providerId])

  const setDraft = useCallback((value: string) => {
    const next = value.slice(0, 12_000)
    draftRef.current = next
    localAiProviderDraftCache.remember(activeIdentity.current, next)
    setDraftValue(next)
  }, [])

  return { draft, draftRef, draftTouched, setDraft, setDraftTouched }
}

import {
  legacyDomEventNameForType,
  REALTIME_DOM_EVENTS,
  type RealtimeEvent,
} from './realtimeEvents'

export type RealtimeEventHandler = (event: RealtimeEvent) => void

export function dispatchRealtimeEvent(event: RealtimeEvent) {
  window.dispatchEvent(new CustomEvent(REALTIME_DOM_EVENTS.realtime, { detail: event }))
  window.dispatchEvent(new CustomEvent(legacyDomEventNameForType(event.type), { detail: event.raw }))
}

export function subscribeRealtimeEvents(handler: RealtimeEventHandler) {
  const listener = (domEvent: Event) => {
    const event = (domEvent as CustomEvent<RealtimeEvent>).detail
    if (event?.type) handler(event)
  }
  window.addEventListener(REALTIME_DOM_EVENTS.realtime, listener)
  return () => window.removeEventListener(REALTIME_DOM_EVENTS.realtime, listener)
}

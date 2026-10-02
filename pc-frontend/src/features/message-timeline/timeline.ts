import '../../../../server/src/assets/message_timeline.js'

export interface TimelineMessage { id: string; created_at: string; timeline_cursor?: string; revision?: number; recalled_at?: string | null }
export interface TimelineScope { kind: 'friend' | 'group' | 'ai' | 'channel'; id: string; project?: string }
export type Direction = 'latest' | 'older' | 'newer' | 'around' | 'sync' | 'window'
export interface ReadingTarget { resolved_id: string; status: string; fraction?: number }
export interface TimelinePage<T> { schema: string; messages: T[]; removed_ids: string[]; before: string | null; sync: string | null; has_more: boolean; reset: boolean; target?: ReadingTarget }
export interface TimelineSnapshot<T> { messages: T[]; before: string | null; sync: string | null; hasOlder: boolean; hasNewer: boolean; following: boolean; unread: number; version: number; reset?: boolean }
export interface Timeline<T> {
  reset(): void
  snapshot(): TimelineSnapshot<T>
  apply(page: TimelinePage<T>, direction: Direction): TimelineSnapshot<T>
  query(scope: TimelineScope, direction: Direction): string
  follow(value: boolean): void
  upgrade(version?: number): void
}
declare global {
  var ElonMessageTimeline: { create<T>(options?: { maxMessages?: number; maxBytes?: number }): Timeline<T> }
}
export const createTimeline = <T>() => ElonMessageTimeline.create<T>({ maxMessages: 300, maxBytes: 3_000_000 })

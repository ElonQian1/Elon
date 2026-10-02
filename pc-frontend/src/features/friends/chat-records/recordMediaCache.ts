const MIB = 1024 * 1024
export const RECORD_MEDIA_LIMIT = 12 * MIB
interface Entry { key: string; blob: Blob; bytes: number; saved: number; accessed: number }
type Metadata = Omit<Entry, 'blob'>
interface Policy { memoryBytes: number; diskBytes: number; maxEntries: number; maxAge: number; database: string; now: () => number }
const defaults: Policy = { memoryBytes: 32 * MIB, diskBytes: 128 * MIB, maxEntries: 128, maxAge: 7 * 86400_000, database: 'elon-record-media-v1', now: Date.now }

/** Immutable attachment bytes only. Authorization is checked by the reader before use. */
export class RecordMediaCache {
  private memory = new Map<string, Entry>()
  private database?: Promise<IDBDatabase | undefined>
  private revision = 0
  private policy: Policy
  constructor(policy: Partial<Policy> = {}) { this.policy = { ...defaults, ...policy } }
  private fresh(entry: Metadata | undefined): entry is Metadata {
    return !!entry && Number.isFinite(entry.bytes)
      && entry.bytes > 0 && entry.bytes <= RECORD_MEDIA_LIMIT
      && Number.isFinite(entry.saved) && this.policy.now() >= entry.saved
      && this.policy.now() - entry.saved < this.policy.maxAge
  }
  private valid(entry: Entry | undefined): entry is Entry {
    return this.fresh(entry) && (entry as Entry).blob instanceof Blob && entry.bytes === (entry as Entry).blob.size
  }
  private metadata(entry: Entry): Metadata { return { key: entry.key, bytes: entry.bytes, saved: entry.saved, accessed: entry.accessed } }
  private remember(entry: Entry) {
    this.memory.delete(entry.key); this.memory.set(entry.key, entry)
    let bytes = [...this.memory.values()].reduce((sum, item) => sum + item.bytes, 0)
    for (const [key, item] of this.memory) {
      if (bytes <= this.policy.memoryBytes && this.memory.size <= this.policy.maxEntries) break
      this.memory.delete(key); bytes -= item.bytes
    }
  }
  private open(): Promise<IDBDatabase | undefined> {
    if (this.database) return this.database
    this.database = new Promise(resolve => {
      let done = false
      const finish = (db?: IDBDatabase) => { if (!done) { done = true; clearTimeout(timer); resolve(db) } else db?.close() }
      const timer = setTimeout(() => finish(), 800)
      try {
        const request = indexedDB.open(this.policy.database, 1)
        request.onupgradeneeded = () => {
          request.result.createObjectStore('media', { keyPath: 'key' })
          request.result.createObjectStore('metadata', { keyPath: 'key' })
        }
        request.onerror = request.onblocked = () => finish()
        request.onsuccess = () => {
          request.result.onversionchange = () => { request.result.close(); this.database = undefined }
          finish(request.result)
        }
      } catch { finish() }
    })
    return this.database
  }
  private async disk<T>(mode: IDBTransactionMode, fallback: T, work: (store: IDBObjectStore, result: (value: T) => void, metadata: IDBObjectStore) => void): Promise<T> {
    const db = await this.open()
    if (!db) return fallback
    return new Promise(resolve => {
      let tx: IDBTransaction | undefined, value = fallback, done = false
      const finish = (result: T) => { if (!done) { done = true; clearTimeout(timer); resolve(result) } }
      const timer = setTimeout(() => { try { tx?.abort() } catch { /* Already complete. */ } finish(fallback) }, 1000)
      try {
        tx = db.transaction(['media', 'metadata'], mode)
        tx.oncomplete = () => finish(value)
        tx.onerror = tx.onabort = () => finish(fallback)
        work(tx.objectStore('media'), next => { value = next }, tx.objectStore('metadata'))
      } catch { finish(fallback) }
    })
  }
  async get(key: string): Promise<Blob | undefined> {
    const revision = this.revision
    const hot = this.memory.get(key)
    if (this.valid(hot)) { hot.accessed = this.policy.now(); this.remember(hot); return hot.blob }
    this.memory.delete(key)
    const entry = await this.disk<Entry | undefined>('readwrite', undefined, (store, result, metadata) => {
      const request = store.get(key)
      request.onsuccess = () => {
        const item = request.result as Entry | undefined
        if (this.valid(item)) { item.accessed = this.policy.now(); metadata.put(this.metadata(item)); result(item) }
        else if (item) { store.delete(key); metadata.delete(key) }
      }
    })
    if (revision === this.revision && this.valid(entry)) { this.remember(entry); return entry.blob }
  }
  async put(key: string, blob: Blob, current: () => boolean = () => true): Promise<void> {
    if (!current() || !blob.size || blob.size > RECORD_MEDIA_LIMIT) return
    const entry: Entry = { key, blob, bytes: blob.size, saved: this.policy.now(), accessed: this.policy.now() }
    this.remember(entry)
    await this.disk('readwrite', undefined, (store, _result, metadata) => {
      if (!current()) return
      store.put(entry)
      metadata.put(this.metadata(entry))
      // Eviction scans only small metadata, never all cached image/video bodies.
      const request = metadata.getAll()
      request.onsuccess = () => {
        const entries = (request.result as Metadata[]).sort((a, b) => b.accessed - a.accessed)
        let bytes = 0, count = 0
        for (const item of entries) {
          const key = item.key
          if (!this.fresh(item)) { store.delete(key); metadata.delete(key); continue }
          bytes += item.bytes; count++
          if (bytes > this.policy.diskBytes || count > this.policy.maxEntries) { store.delete(item.key); metadata.delete(item.key) }
        }
      }
    })
  }
  async clear(prefix = ''): Promise<void> {
    this.revision++
    for (const key of this.memory.keys()) if (key.startsWith(prefix)) this.memory.delete(key)
    await this.disk('readwrite', undefined, (store, _result, metadata) => {
      const request = store.openKeyCursor()
      request.onsuccess = () => {
        const cursor = request.result
        if (!cursor) return
        if (String(cursor.key).startsWith(prefix)) { store.delete(cursor.primaryKey); metadata.delete(cursor.primaryKey) }
        cursor.continue()
      }
    })
  }
}

export const recordMediaCache = new RecordMediaCache()

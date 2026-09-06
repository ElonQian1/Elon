import { AssetAccessError } from './transport.js';

const MAX_PROGRESS_IDS = 10000;

/** Remembers only duplicate-detection identifiers for one verified paging chain. */
export class PaginationChain {
  #ids = new Set();
  #cursors = new Set();

  clear() { this.#ids.clear(); this.#cursors.clear(); }

  accept(page, cursor) {
    if (!cursor || !page.progress) this.clear();
    if (!page.progress) return;
    const identifiers = page.progress.requests.map(request => request.request_id);
    const next = page.progress.next_cursor;
    if (new Set(identifiers).size !== identifiers.length ||
        identifiers.some(id => this.#ids.has(id)) ||
        (next !== null && this.#cursors.has(next))) {
      throw new AssetAccessError('invalid_response');
    }
    if (this.#ids.size + identifiers.length > MAX_PROGRESS_IDS) {
      throw new AssetAccessError('pagination_limit');
    }
    for (const id of identifiers) this.#ids.add(id);
    if (next !== null) this.#cursors.add(next);
  }
}

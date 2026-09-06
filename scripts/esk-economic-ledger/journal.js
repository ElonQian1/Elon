'use strict';

const { canonical } = require('../esk-paid-reconciliation/identity');
const { U128_MAX } = require('./input');

function projectJournal(events, lotById, issues) {
  const identities = new Map();
  const eventIds = new Set();
  const requests = new Map();
  const usedRequestIds = new Set();
  const allocated = new Map();
  const purposes = { investment: 0n, guarantee_reserve: 0n, profit_distribution: 0n };
  const counts = { unique_events: 0, replayed_events: 0, proposals: 0,
    pending_proposals: 0, canceled_proposals: 0 };
  let nextSequence = 1;
  for (const event of events) {
    const canonicalEvent = canonical(event);
    if (identities.has(event.idempotency_key)) {
      if (identities.get(event.idempotency_key) === canonicalEvent) counts.replayed_events += 1;
      else issues.add('IDEMPOTENCY_CONFLICT');
      continue;
    }
    identities.set(event.idempotency_key, canonicalEvent);
    if (eventIds.has(event.event_id)) { issues.add('EVENT_ID_CONFLICT'); continue; }
    eventIds.add(event.event_id);
    counts.unique_events += 1;
    if (event.sequence !== nextSequence) {
      issues.add('EVENT_SEQUENCE_INVALID');
      nextSequence += 1;
      continue;
    }
    nextSequence += 1;
    if (event.operation === 'cancel') {
      const previous = requests.get(event.request_id);
      if (!previous) { issues.add('CANCEL_REQUEST_UNKNOWN'); continue; }
      if (previous.status === 'CANCELED') { issues.add('REQUEST_ALREADY_CANCELED'); continue; }
      previous.status = 'CANCELED';
      allocated.set(previous.lot_id, allocated.get(previous.lot_id) - previous.amount);
      purposes[previous.purpose] -= previous.amount;
      counts.pending_proposals -= 1;
      counts.canceled_proposals += 1;
      continue;
    }
    if (usedRequestIds.has(event.request_id)) { issues.add('REQUEST_ID_CONFLICT'); continue; }
    usedRequestIds.add(event.request_id);
    const lot = lotById.get(event.lot_id);
    if (!lot) { issues.add('PROPOSAL_LOT_UNKNOWN'); continue; }
    if (event.purpose === 'profit_distribution' && lot.origin !== 'realized_profit') {
      issues.add('PROFIT_SOURCE_INVALID');
      continue;
    }
    const amount = BigInt(event.amount_base_units);
    const active = (allocated.get(event.lot_id) || 0n) + amount;
    if (active > BigInt(lot.amount_base_units)) {
      issues.add('LOT_OVERALLOCATED');
      continue;
    }
    allocated.set(event.lot_id, active);
    purposes[event.purpose] += amount;
    if (Object.values(purposes).reduce((sum, value) => sum + value, 0n) > U128_MAX) {
      issues.add('TOTAL_OVERFLOW');
    }
    requests.set(event.request_id, { lot_id: event.lot_id, purpose: event.purpose,
      amount, status: 'PENDING' });
    counts.proposals += 1;
    counts.pending_proposals += 1;
  }
  if (!events.some((event) => event.operation === 'propose')) issues.add('JOURNAL_NO_PROPOSALS');
  return { counts, purposes };
}

module.exports = { projectJournal };

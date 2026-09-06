'use strict';

const assert = require('node:assert/strict');
const test = require('node:test');
const { batch, sponsorOnly, clone, proposal, cancellation, evaluate, U128_MAX,
  assertReport, assertIssues } = require('./helpers');

test('exact event replay is checked before sequence and never duplicates allocated amounts', () => {
  const input = batch();
  const baseline = evaluate(input);
  input.journal.splice(1, 0, clone(input.journal[0]));
  input.journal.push(clone(input.journal[0]));
  const report = evaluate(input);
  assertReport(report);
  assert.equal(report.review_status, 'consistent');
  assert.deepEqual(report.totals, baseline.totals);
  assert.equal(report.counts.journal_entries, 6);
  assert.equal(report.counts.unique_events, 4);
  assert.equal(report.counts.replayed_events, 2);
  assert.equal(report.counts.proposals, 4);
});

test('a cancellation releases only the draft allocation and can itself be replayed exactly', () => {
  const input = batch();
  input.journal.push(cancellation(5, 'request-1'));
  input.journal.push(clone(input.journal[4]));
  const report = evaluate(input);
  assertReport(report);
  assert.equal(report.review_status, 'consistent');
  assert.equal(report.totals.investment_base_units, '0');
  assert.equal(report.totals.guarantee_reserve_base_units, '70');
  assert.equal(report.totals.unallocated_base_units, '100');
  assert.equal(report.counts.unique_events, 5);
  assert.equal(report.counts.replayed_events, 1);
  assert.equal(report.counts.pending_proposals, 3);
  assert.equal(report.counts.canceled_proposals, 1);
});

test('released capacity supports a new request without resurrecting the canceled request', () => {
  const input = batch();
  input.journal.push(cancellation(5, 'request-1'), proposal(6, 'purchase', 'investment', '80'));
  const report = evaluate(input);
  assert.equal(report.review_status, 'consistent');
  assert.equal(report.totals.investment_base_units, '80');
  assert.equal(report.counts.proposals, 5);
  assert.equal(report.counts.pending_proposals, 4);
  assert.equal(report.counts.canceled_proposals, 1);
  input.journal[5].request_id = 'request-1';
  assertIssues(input, 'REQUEST_ID_CONFLICT');
});

test('changing any content under an existing idempotency key is a conflict', () => {
  for (const mutation of [event => { event.amount_base_units = '81'; },
    event => { event.sequence = 5; }, event => { event.request_id = 'changed-request'; },
    event => { event.event_id = 'changed-event'; }]) {
    const input = batch();
    const conflict = clone(input.journal[0]);
    mutation(conflict);
    input.journal.push(conflict);
    assertIssues(input, 'IDEMPOTENCY_CONFLICT');
  }
});

test('event IDs and proposal IDs remain unique independently of idempotency keys', () => {
  const eventConflict = batch();
  eventConflict.journal.push({ ...proposal(5, 'profit', 'investment', '1'), event_id: 'event-1' });
  assertIssues(eventConflict, 'EVENT_ID_CONFLICT');
  const requestConflict = batch();
  requestConflict.journal.push(proposal(5, 'profit', 'investment', '1', 'request-1'));
  assertIssues(requestConflict, 'REQUEST_ID_CONFLICT');
});

test('unique event sequences must start at one and stay contiguous in supplied order', () => {
  const start = batch();
  start.journal[0].sequence = 2;
  assertIssues(start, 'EVENT_SEQUENCE_INVALID');
  const gap = batch();
  gap.journal[3].sequence = 5;
  assertIssues(gap, 'EVENT_SEQUENCE_INVALID');
  const reverse = batch();
  [reverse.journal[0], reverse.journal[1]] = [reverse.journal[1], reverse.journal[0]];
  assertIssues(reverse, 'EVENT_SEQUENCE_INVALID');
});

test('cancellations cannot target absent, future, or previously canceled requests', () => {
  const unknown = batch();
  unknown.journal.push(cancellation(5, 'unknown-request'));
  assertIssues(unknown, 'CANCEL_REQUEST_UNKNOWN');
  const future = sponsorOnly();
  future.journal = [cancellation(1, 'later'), proposal(2, 'sponsor', 'investment', '10', 'later')];
  assertIssues(future, 'CANCEL_REQUEST_UNKNOWN');
  const repeated = batch();
  repeated.journal.push(cancellation(5, 'request-1'), cancellation(6, 'request-1'));
  assertIssues(repeated, 'REQUEST_ALREADY_CANCELED');
});

test('investment and reserve share the same per-lot capacity at every step', () => {
  const duplicateUse = batch();
  duplicateUse.journal[1].amount_base_units = '21';
  assertIssues(duplicateUse, 'LOT_OVERALLOCATED');
  duplicateUse.journal.push(cancellation(5, 'request-2'));
  assertIssues(duplicateUse, 'LOT_OVERALLOCATED');
  const otherLotsCannotCover = batch();
  otherLotsCannotCover.journal = [proposal(1, 'purchase', 'investment', '110')];
  assertIssues(otherLotsCannotCover, 'LOT_OVERALLOCATED');
});

test('purchase proceeds and sponsor capital cannot be distributed as realized profit', () => {
  for (const lot of ['purchase', 'sponsor']) {
    const input = batch();
    input.journal = [proposal(1, lot, 'profit_distribution', '1')];
    assertIssues(input, 'PROFIT_SOURCE_INVALID');
  }
  const unknown = batch();
  unknown.journal = [proposal(1, 'absent-lot', 'investment', '1')];
  assertIssues(unknown, 'PROPOSAL_LOT_UNKNOWN');
});

test('large integers retain one-unit differences beyond Number precision', () => {
  const input = sponsorOnly('9007199254740993');
  input.journal[0].amount_base_units = '9007199254740992';
  const report = evaluate(input);
  assert.equal(report.review_status, 'consistent');
  assert.equal(report.totals.funding_total_base_units, '9007199254740993');
  assert.equal(report.totals.investment_base_units, '9007199254740992');
  assert.equal(report.totals.unallocated_base_units, '1');
});

test('u128 maximum is usable but aggregated funding cannot exceed it', () => {
  const input = sponsorOnly(U128_MAX.toString());
  const report = evaluate(input);
  assert.equal(report.review_status, 'consistent');
  assert.equal(report.totals.investment_base_units, U128_MAX.toString());
  assert.equal(report.totals.unallocated_base_units, '0');
  input.funding_lots.push({ ...input.funding_lots[0], lot_id: 'one-more',
    external_payment_reference: 'd'.repeat(64), amount_base_units: '1' });
  assertIssues(input, 'TOTAL_OVERFLOW');
});

test('empty batches or batches without any proposed purpose are explicitly incomplete', () => {
  const empty = batch();
  empty.funding_lots = [];
  empty.obligation_links = [];
  empty.journal = [];
  assertIssues(empty, 'FUNDING_LOTS_EMPTY', 'JOURNAL_NO_PROPOSALS');
  const unused = batch();
  unused.journal = [];
  assertIssues(unused, 'JOURNAL_NO_PROPOSALS');
});

'use strict';

const { MAX_BYTES, parseStrictJson } = require('../esk-paid-reconciliation/strict-json');
const { preview } = require('../esk-paid-reconciliation/preview');
const { evaluatePolicyBuffer } = require('../esk-early-support-policy/contract');

const MAX_INPUT_BYTES = MAX_BYTES;
const U128_MAX = (1n << 128n) - 1n;
const ID = /^[A-Za-z0-9._:-]{1,80}$/;
const KEY = /^[A-Za-z0-9._:-]{1,128}$/;
const DIGEST = /^[0-9a-f]{64}$/;
const ERRORS = new Set([
  'INVALID_INPUT', 'INVALID_STRUCTURE', 'INVALID_VALUE', 'INVALID_AMOUNT',
  'INVALID_POLICY_DRAFT', 'INVALID_PAID_RECONCILIATION', 'INVALID_ARGUMENTS',
  'UNSAFE_KEY', 'INTERNAL_ERROR', 'INVALID_JSON', 'DUPLICATE_JSON_KEY',
  'INPUT_TOO_LARGE', 'INVALID_UTF8', 'INPUT_TOO_DEEP', 'INPUT_TIMEOUT',
]);

class LedgerInputError extends Error {
  constructor(code) {
    const safe = ERRORS.has(code) ? code : 'INTERNAL_ERROR';
    super(safe);
    this.name = 'LedgerInputError';
    this.code = safe;
  }
}

function ensure(condition, code = 'INVALID_VALUE') {
  if (!condition) throw new LedgerInputError(code);
}

function exact(value, keys) {
  ensure(value !== null && typeof value === 'object' && !Array.isArray(value), 'INVALID_STRUCTURE');
  ensure(Object.keys(value).length === keys.length
    && keys.every((key) => Object.hasOwn(value, key)), 'INVALID_STRUCTURE');
}

function matches(value, pattern) {
  if (typeof value !== 'string') return false;
  const result = pattern.exec(value);
  return result !== null && result[0] === value;
}

function text(value, pattern = ID) { ensure(matches(value, pattern)); }
function integer(value, maximum, minimum = 0) {
  ensure(Number.isSafeInteger(value) && !Object.is(value, -0)
    && value >= minimum && value <= maximum);
}
function list(value, maximum) {
  ensure(Array.isArray(value) && value.length <= maximum, 'INVALID_STRUCTURE');
}
function amount(value) {
  ensure(typeof value === 'string' && value.length <= 39
    && matches(value, /^[1-9][0-9]*$/), 'INVALID_AMOUNT');
  const result = BigInt(value);
  ensure(result <= U128_MAX, 'INVALID_AMOUNT');
  return result;
}

function inspect(value) {
  if (typeof value === 'string') {
    for (const character of value) {
      const point = character.codePointAt(0);
      ensure(point < 0xd800 || point > 0xdfff);
    }
  } else if (value && typeof value === 'object') {
    for (const key of Object.keys(value)) {
      ensure(!['__proto__', 'prototype', 'constructor'].includes(key), 'UNSAFE_KEY');
      inspect(key);
      inspect(value[key]);
    }
  }
}

function validateCollections(input, source) {
  list(input.funding_lots, 200);
  for (const lot of input.funding_lots) {
    exact(lot, ['lot_id', 'origin', 'external_payment_reference', 'transfer_index', 'amount_base_units']);
    text(lot.lot_id);
    ensure(['esk_purchase', 'sponsor_capital', 'realized_profit'].includes(lot.origin));
    // These lexical rules match the existing payment source contract. Identity
    // normalization and business eligibility remain in its shared implementation.
    text(lot.external_payment_reference, source.reference_format === 'hex32'
      ? /^(?:0[xX])?[0-9a-fA-F]{64}$/ : KEY);
    integer(lot.transfer_index, 2147483647);
    amount(lot.amount_base_units);
  }
  list(input.obligation_links, 200);
  for (const link of input.obligation_links) {
    exact(link, ['obligation_id', 'purchase_lot_id', 'policy_digest', 'status',
      'protected_principal_base_units', 'minimum_return_base_units']);
    text(link.obligation_id);
    text(link.purchase_lot_id);
    text(link.policy_digest, DIGEST);
    ensure(link.status === 'PENDING' && link.protected_principal_base_units === null
      && link.minimum_return_base_units === null);
  }
  list(input.journal, 500);
  for (const event of input.journal) {
    ensure(event && ['propose', 'cancel'].includes(event.operation));
    const common = ['sequence', 'event_id', 'idempotency_key', 'operation', 'request_id'];
    exact(event, event.operation === 'propose'
      ? [...common, 'lot_id', 'purpose', 'amount_base_units'] : common);
    integer(event.sequence, 500, 1);
    text(event.event_id);
    text(event.idempotency_key, KEY);
    text(event.request_id);
    if (event.operation === 'propose') {
      text(event.lot_id);
      ensure(['investment', 'guarantee_reserve', 'profit_distribution'].includes(event.purpose));
      amount(event.amount_base_units);
    }
  }
}

function readLedgerInput(buffer) {
  let input;
  try { input = parseStrictJson(buffer); } catch (error) { throw new LedgerInputError(error.code); }
  inspect(input);
  exact(input, ['schema', 'mode', 'policy_draft', 'paid_reconciliation',
    'funding_lots', 'obligation_links', 'journal']);
  ensure(input.schema === 'elon.esk.economic_ledger_preview_input.v1' && input.mode === 'offline_draft');
  let policyReview;
  try {
    policyReview = evaluatePolicyBuffer(Buffer.from(JSON.stringify(input.policy_draft), 'utf8'));
  } catch { throw new LedgerInputError('INVALID_POLICY_DRAFT'); }
  const paidReview = preview(input.paid_reconciliation);
  ensure(paidReview.status !== 'invalid_input', 'INVALID_PAID_RECONCILIATION');
  validateCollections(input, input.paid_reconciliation.source);
  return { input, policyReview, paidReview };
}

module.exports = { MAX_INPUT_BYTES, U128_MAX, LedgerInputError, readLedgerInput };

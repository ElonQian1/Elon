'use strict';

const { MAX_BYTES, InputError, parseStrictJson } = require('./strict-json');

const I64_MAX = 9223372036854775807n;
const U128_MAX = (1n << 128n) - 1n;
const DIGEST = /^[0-9a-f]{64}$/;
const LABEL = /^[A-Za-z0-9._:-]{1,80}$/;
const SOURCE_ID = /^[a-z0-9._:-]{1,96}$/;
const ASSET_REFERENCE = /^[A-Za-z0-9._:-]{1,160}$/;
const OPAQUE_REFERENCE = /^[A-Za-z0-9._:-]{1,128}$/;
const HEX_REFERENCE = /^(?:0[xX])?[0-9a-fA-F]{64}$/;

function ensure(condition, code = 'INVALID_INPUT') {
  if (!condition) throw new InputError(code);
}

function exactKeys(value, keys) {
  ensure(value !== null && typeof value === 'object' && !Array.isArray(value));
  const prototype = Object.getPrototypeOf(value);
  ensure(prototype === Object.prototype || prototype === null);
  const own = Reflect.ownKeys(value);
  ensure(own.length === keys.length && own.every((key) => keys.includes(key)));
  for (const key of keys) {
    const descriptor = Object.getOwnPropertyDescriptor(value, key);
    ensure(descriptor && Object.hasOwn(descriptor, 'value') && descriptor.enumerable);
  }
}

function matches(value, pattern) {
  if (typeof value !== 'string') return false;
  const match = pattern.exec(value);
  return match !== null && match[0] === value;
}

function text(value, pattern) {
  ensure(matches(value, pattern));
}

function timestamp(value) {
  ensure(typeof value === 'string'
    && /^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}\.\d{3}Z$/.test(value));
  const parsed = new Date(value);
  ensure(Number.isFinite(parsed.getTime()) && parsed.toISOString() === value);
}

function integer(value, maximum) {
  ensure(Number.isSafeInteger(value) && !Object.is(value, -0)
    && value >= 0 && value <= maximum);
}

function list(value, maximum, minimum = 0) {
  ensure(Array.isArray(value) && value.length >= minimum && value.length <= maximum);
  // Reject sparse or property-extended arrays when called directly, not just via JSON.
  ensure(Reflect.ownKeys(value).length === value.length + 1);
  for (let index = 0; index < value.length; index += 1) {
    const descriptor = Object.getOwnPropertyDescriptor(value, String(index));
    ensure(descriptor && Object.hasOwn(descriptor, 'value') && descriptor.enumerable);
  }
}

function baseUnits(value, maximum, allowZero) {
  ensure(typeof value === 'string' && value.length <= 39
    && matches(value, /^(?:0|[1-9][0-9]*)$/), 'INVALID_AMOUNT');
  const units = BigInt(value);
  ensure(units <= maximum && (allowZero || units > 0n), 'INVALID_AMOUNT');
  return units;
}

function parseAmount(value, decimals) {
  ensure(Number.isSafeInteger(decimals) && decimals >= 0 && decimals <= 18
    && !Object.is(decimals, -0), 'INVALID_AMOUNT');
  ensure(typeof value === 'string' && value.length <= 59
    && matches(value, /^(?:0|[1-9][0-9]*)(?:\.[0-9]+)?$/), 'INVALID_AMOUNT');
  const [whole, fraction = ''] = value.split('.');
  ensure(fraction.length <= decimals, 'INVALID_AMOUNT');
  const units = BigInt(whole) * (10n ** BigInt(decimals))
    + BigInt((fraction + '0'.repeat(decimals - fraction.length)) || '0');
  ensure(units > 0n && units <= U128_MAX, 'INVALID_AMOUNT');
  return units;
}

function optionalText(value, pattern) {
  if (value !== null) text(value, pattern);
}

function validateSource(source) {
  exactKeys(source, [
    'namespace', 'network', 'asset_symbol', 'asset_reference', 'decimals', 'reference_format',
  ]);
  text(source.namespace, SOURCE_ID);
  text(source.network, SOURCE_ID);
  ensure(source.asset_symbol === 'USDT');
  text(source.asset_reference, ASSET_REFERENCE);
  integer(source.decimals, 18);
  ensure(['hex32', 'opaque'].includes(source.reference_format));
}

function validateSnapshot(snapshot) {
  exactKeys(snapshot, [
    'snapshot_id', 'observed_at', 'source_fingerprint', 'history_complete', 'used_payment_keys',
  ]);
  text(snapshot.snapshot_id, LABEL);
  timestamp(snapshot.observed_at);
  text(snapshot.source_fingerprint, DIGEST);
  ensure(typeof snapshot.history_complete === 'boolean');
  list(snapshot.used_payment_keys, 10000);
  for (const key of snapshot.used_payment_keys) text(key, DIGEST);
}

function validateSaleBatch(batch) {
  exactKeys(batch, [
    'sale_batch_id', 'payment_base_units_per_lot', 'esk_base_units_per_lot',
    'disclosure_revision', 'terms_digest',
  ]);
  text(batch.sale_batch_id, LABEL);
  baseUnits(batch.payment_base_units_per_lot, U128_MAX, false);
  baseUnits(batch.esk_base_units_per_lot, I64_MAX, false);
  text(batch.disclosure_revision, LABEL);
  text(batch.terms_digest, DIGEST);
}

function validateRow(row, source) {
  exactKeys(row, [
    'row_id', 'external_payment_reference', 'transfer_index', 'payment_amount',
    'opaque_subject', 'payment_status', 'commercial_purpose', 'esk_base_units',
    'sale_batch_id', 'disclosure_revision', 'consent_digest', 'approval_digest',
  ]);
  text(row.row_id, LABEL);
  text(row.external_payment_reference,
    source.reference_format === 'hex32' ? HEX_REFERENCE : OPAQUE_REFERENCE);
  integer(row.transfer_index, 2147483647);
  parseAmount(row.payment_amount, source.decimals);
  text(row.opaque_subject, DIGEST);
  ensure(['confirmed', 'pending', 'reversed'].includes(row.payment_status));
  ensure(['esk_purchase', 'service_purchase', 'quant_subscription', 'unconfirmed']
    .includes(row.commercial_purpose));
  baseUnits(row.esk_base_units, I64_MAX, true);
  optionalText(row.sale_batch_id, LABEL);
  optionalText(row.disclosure_revision, LABEL);
  optionalText(row.consent_digest, DIGEST);
  optionalText(row.approval_digest, DIGEST);
}

// Shape validation is intentionally independent of payment authenticity, user
// identity, approvals, source coverage, and duplicate/business-policy checks.
function validateInput(input) {
  exactKeys(input, [
    'schema', 'batch_id', 'as_of', 'source', 'snapshot', 'users', 'sale_batches', 'rows',
  ]);
  ensure(input.schema === 'yilong.esk.paid_reconciliation_input.v1');
  text(input.batch_id, LABEL);
  timestamp(input.as_of);
  validateSource(input.source);
  validateSnapshot(input.snapshot);
  list(input.users, 1000);
  for (const user of input.users) {
    exactKeys(user, ['opaque_subject', 'target_user_ref']);
    text(user.opaque_subject, DIGEST);
    text(user.target_user_ref, DIGEST);
  }
  list(input.sale_batches, 100);
  for (const batch of input.sale_batches) validateSaleBatch(batch);
  list(input.rows, 1000, 1);
  for (const row of input.rows) validateRow(row, input.source);
  return input;
}

function parseInput(buffer) {
  return validateInput(parseStrictJson(buffer));
}

module.exports = { MAX_BYTES, I64_MAX, InputError, parseInput, validateInput, parseAmount };

'use strict';

const { paymentKey } = require('../esk-paid-reconciliation/identity');
const { U128_MAX } = require('./input');

function checkSources(input, paidReview, policyDigest, issues) {
  const { funding_lots: lots, obligation_links: links, paid_reconciliation: paid } = input;
  if (!lots.length) issues.add('FUNDING_LOTS_EMPTY');
  const lotById = new Map();
  const identities = new Set();
  const paidByKey = new Map();
  for (const row of paidReview.rows) {
    paidByKey.set(row.payment_key, [...(paidByKey.get(row.payment_key) || []), row]);
  }
  const history = new Set(paid.snapshot.used_payment_keys);
  const funding = { esk_purchase: 0n, sponsor_capital: 0n, realized_profit: 0n, total: 0n };
  for (const lot of lots) {
    if (lotById.has(lot.lot_id)) issues.add('DUPLICATE_LOT_ID');
    else lotById.set(lot.lot_id, lot);
    const key = paymentKey(paid.source, lot);
    if (identities.has(key)) issues.add('DUPLICATE_PAYMENT_KEY');
    identities.add(key);
    if (history.has(key)) issues.add('PAYMENT_ALREADY_USED');
    const rows = paidByKey.get(key) || [];
    if (lot.origin === 'esk_purchase') {
      if (!rows.length) issues.add('PURCHASE_PAYMENT_NOT_FOUND');
      else if (rows.length !== 1 || rows[0].status !== 'review_ready') {
        issues.add('PURCHASE_PAYMENT_NOT_REVIEW_READY');
      } else if (rows[0].payment_base_units !== lot.amount_base_units) {
        issues.add('PURCHASE_AMOUNT_MISMATCH');
      }
    } else if (rows.length) issues.add('NON_PURCHASE_REUSES_PAID_PAYMENT');
    const amount = BigInt(lot.amount_base_units);
    funding[lot.origin] += amount;
    funding.total += amount;
    if (funding.total > U128_MAX) issues.add('TOTAL_OVERFLOW');
  }
  const obligationIds = new Set();
  const linkedPurchases = new Set();
  for (const link of links) {
    if (obligationIds.has(link.obligation_id)) issues.add('OBLIGATION_DUPLICATE_ID');
    obligationIds.add(link.obligation_id);
    const lot = lotById.get(link.purchase_lot_id);
    if (!lot || lot.origin !== 'esk_purchase') issues.add('OBLIGATION_PURCHASE_INVALID');
    if (linkedPurchases.has(link.purchase_lot_id)) issues.add('OBLIGATION_DUPLICATE_PURCHASE');
    linkedPurchases.add(link.purchase_lot_id);
    if (link.policy_digest !== policyDigest) issues.add('OBLIGATION_POLICY_MISMATCH');
  }
  for (const lot of lots) {
    if (lot.origin === 'esk_purchase' && !linkedPurchases.has(lot.lot_id)) {
      issues.add('OBLIGATION_MISSING');
    }
  }
  return { lotById, funding };
}

module.exports = { checkSources };

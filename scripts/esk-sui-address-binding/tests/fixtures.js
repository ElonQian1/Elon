const SUBJECT = `sha256:${'a'.repeat(64)}`

function request(address) {
  return {
    schema: 'yilong.esk.sui.address_binding_challenge_request.v1',
    network: 'testnet',
    purpose: 'user_asset_migration',
    subject_commitment: SUBJECT,
    address,
    ttl_seconds: 600,
  }
}

module.exports = { SUBJECT, request }

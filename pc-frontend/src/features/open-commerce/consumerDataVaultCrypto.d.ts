import type { ConsumerDataVaultEnvelope } from './openCommerceClientTypes'

export const CONSUMER_DATA_VAULT_ENVELOPE_SCHEMA:
  'open_commerce.consumer_data_vault_envelope.v1'
export const CONSUMER_DATA_VAULT_PLAINTEXT_SCHEMA:
  'open_commerce.consumer_data_vault_plaintext.v1'
export const CONSUMER_DATA_VAULT_ITERATIONS: 310000

export function encryptConsumerDataVaultItem(
  recordId: string,
  revision: number,
  content: string,
  passphrase: string,
): Promise<ConsumerDataVaultEnvelope>

export function decryptConsumerDataVaultItem(
  envelope: ConsumerDataVaultEnvelope,
  passphrase: string,
): Promise<string>

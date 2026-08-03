import { api } from '../../api/client'
import type {
  DeveloperWebhookCredential,
  DeveloperWebhookDelivery,
  DeveloperWebhookDeliveryList,
  DeveloperWebhookHistoryReplayResult,
  DeveloperWebhookHealthSummary,
  DeveloperWebhookSubscription,
  DeveloperWebhookSubscriptionList,
} from './openCommerceClientTypes'

function projectBase(projectId: string) {
  return `/api/projects/${encodeURIComponent(projectId)}/open-commerce`
}

export const developerWebhookClientApi = {
  listDeveloperWebhooks: (projectId: string, appRecordId: string) =>
    api.get<DeveloperWebhookSubscriptionList>(
      `${projectBase(projectId)}/developer-apps/${encodeURIComponent(appRecordId)}/webhooks`,
    ),

  getDeveloperWebhookHealth: (projectId: string, appRecordId: string) =>
    api.get<DeveloperWebhookHealthSummary>(
      `${projectBase(projectId)}/developer-apps/${encodeURIComponent(appRecordId)}/webhook-health`,
    ),

  createDeveloperWebhook: (
    projectId: string,
    appRecordId: string,
    callbackUrl: string,
    environment: 'sandbox' | 'production',
    deliverOnSucceeded: boolean,
    deliverOnFailed: boolean,
  ) =>
    api.post<DeveloperWebhookCredential>(
      `${projectBase(projectId)}/developer-apps/${encodeURIComponent(appRecordId)}/webhooks`,
      {
        callback_url: callbackUrl,
        environment,
        deliver_on_succeeded: deliverOnSucceeded,
        deliver_on_failed: deliverOnFailed,
      },
    ),

  disableDeveloperWebhook: (projectId: string, appRecordId: string, webhookId: string) =>
    api.post<DeveloperWebhookSubscription>(
      `${projectBase(projectId)}/developer-apps/${encodeURIComponent(appRecordId)}/webhooks/${encodeURIComponent(webhookId)}/disable`,
      {},
    ),

  enableDeveloperWebhook: (projectId: string, appRecordId: string, webhookId: string) =>
    api.post<DeveloperWebhookSubscription>(
      `${projectBase(projectId)}/developer-apps/${encodeURIComponent(appRecordId)}/webhooks/${encodeURIComponent(webhookId)}/enable`,
      {},
    ),

  verifyDeveloperWebhook: (projectId: string, appRecordId: string, webhookId: string) =>
    api.post<DeveloperWebhookSubscription>(
      `${projectBase(projectId)}/developer-apps/${encodeURIComponent(appRecordId)}/webhooks/${encodeURIComponent(webhookId)}/verify`,
      {},
    ),

  rotateDeveloperWebhookSecret: (projectId: string, appRecordId: string, webhookId: string) =>
    api.post<DeveloperWebhookCredential>(
      `${projectBase(projectId)}/developer-apps/${encodeURIComponent(appRecordId)}/webhooks/${encodeURIComponent(webhookId)}/rotate-secret`,
      {},
    ),

  listDeveloperWebhookDeliveries: (
    projectId: string,
    appRecordId: string,
    webhookId: string,
  ) => api.get<DeveloperWebhookDeliveryList>(
    `${projectBase(projectId)}/developer-apps/${encodeURIComponent(appRecordId)}/webhooks/${encodeURIComponent(webhookId)}/deliveries`,
  ),

  retryDeveloperWebhookDelivery: (
    projectId: string,
    appRecordId: string,
    webhookId: string,
    deliveryId: string,
  ) => api.post<DeveloperWebhookDelivery>(
    `${projectBase(projectId)}/developer-apps/${encodeURIComponent(appRecordId)}/webhooks/${encodeURIComponent(webhookId)}/deliveries/${encodeURIComponent(deliveryId)}/retry`,
    {},
  ),

  acknowledgeDeveloperWebhookDeadLetter: (
    projectId: string,
    appRecordId: string,
    webhookId: string,
    deliveryId: string,
    reason: string,
  ) => api.post<DeveloperWebhookDelivery>(
    `${projectBase(projectId)}/developer-apps/${encodeURIComponent(appRecordId)}/webhooks/${encodeURIComponent(webhookId)}/deliveries/${encodeURIComponent(deliveryId)}/acknowledge`,
    { reason },
  ),

  replayDeveloperWebhookHistory: (
    projectId: string,
    appRecordId: string,
    webhookId: string,
    afterSequence: number,
    limit: number,
  ) => api.post<DeveloperWebhookHistoryReplayResult>(
    `${projectBase(projectId)}/developer-apps/${encodeURIComponent(appRecordId)}/webhooks/${encodeURIComponent(webhookId)}/replay-history`,
    { after_sequence: afterSequence, limit },
  ),
}

import { nodeApi } from '../localNodeApi'
import type {
  NodeDataRootCleanupResponse,
  NodeDataRootSetResponse,
  NodeDataRootStatus,
} from './types'

const STATUS_PATH = '/api/node-data-root'
const CLEANUP_PATH = '/api/node-data-root/cleanup'

export function fetchNodeDataRoot(adminUrl: string): Promise<NodeDataRootStatus> {
  return nodeApi<NodeDataRootStatus>(adminUrl, STATUS_PATH, {}, 15000)
}

export function saveNodeDataRoot(adminUrl: string, rootPath: string): Promise<NodeDataRootSetResponse> {
  return nodeApi<NodeDataRootSetResponse>(adminUrl, STATUS_PATH, {
    method: 'POST',
    body: JSON.stringify({ root_path: rootPath }),
  }, 30000)
}

export function cleanupNodeDataRoot(
  adminUrl: string,
  apply: boolean,
): Promise<NodeDataRootCleanupResponse> {
  return nodeApi<NodeDataRootCleanupResponse>(adminUrl, CLEANUP_PATH, {
    method: 'POST',
    body: JSON.stringify({ apply }),
  }, apply ? 120000 : 30000)
}

import OpenCommerceMerchantWorkspace from './OpenCommerceMerchantWorkspace'

export default function OpenCommercePanel({
  projectId,
  canEdit,
}: {
  projectId: string
  canEdit: boolean
}) {
  return <OpenCommerceMerchantWorkspace projectId={projectId} canEdit={canEdit} />
}

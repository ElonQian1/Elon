use crate::{ComputePluginSharingPolicyObservedV1, ComputePluginSharingPolicySnapshotV1};

use super::{
    validation::{validate_sharing_observed, validate_sharing_request},
    NODE_ENDPOINT_PLANNING_BOOTSTRAP_SHARING_OBSERVED_V1_SCHEMA,
    NODE_ENDPOINT_PLANNING_BOOTSTRAP_SHARING_OBSERVED_V1_TYPE,
    NODE_ENDPOINT_PLANNING_BOOTSTRAP_SHARING_REQUEST_V1_SCHEMA,
    NODE_ENDPOINT_PLANNING_BOOTSTRAP_SHARING_REQUEST_V1_TYPE,
};

define_planning_bootstrap_message!(
    NodeEndpointPlanningBootstrapSharingRequestV1,
    NodeEndpointPlanningBootstrapSharingRequestV1Fields,
    snapshot: ComputePluginSharingPolicySnapshotV1,
    NODE_ENDPOINT_PLANNING_BOOTSTRAP_SHARING_REQUEST_V1_TYPE,
    NODE_ENDPOINT_PLANNING_BOOTSTRAP_SHARING_REQUEST_V1_SCHEMA,
    1,
    validate_sharing_request
);

define_planning_bootstrap_message!(
    NodeEndpointPlanningBootstrapSharingObservedV1,
    NodeEndpointPlanningBootstrapSharingObservedV1Fields,
    observed: ComputePluginSharingPolicyObservedV1,
    NODE_ENDPOINT_PLANNING_BOOTSTRAP_SHARING_OBSERVED_V1_TYPE,
    NODE_ENDPOINT_PLANNING_BOOTSTRAP_SHARING_OBSERVED_V1_SCHEMA,
    2,
    validate_sharing_observed
);

use crate::{
    ComputePluginInstallPlanPlanningSnapshotObservedV2,
    ComputePluginInstallPlanPlanningSnapshotRequestV2,
};

use super::{
    validation::{validate_snapshot_observed, validate_snapshot_request},
    NODE_ENDPOINT_PLANNING_BOOTSTRAP_SNAPSHOT_OBSERVED_V1_SCHEMA,
    NODE_ENDPOINT_PLANNING_BOOTSTRAP_SNAPSHOT_OBSERVED_V1_TYPE,
    NODE_ENDPOINT_PLANNING_BOOTSTRAP_SNAPSHOT_REQUEST_V1_SCHEMA,
    NODE_ENDPOINT_PLANNING_BOOTSTRAP_SNAPSHOT_REQUEST_V1_TYPE,
};

define_planning_bootstrap_message!(
    NodeEndpointPlanningBootstrapSnapshotRequestV1,
    NodeEndpointPlanningBootstrapSnapshotRequestV1Fields,
    request: ComputePluginInstallPlanPlanningSnapshotRequestV2,
    NODE_ENDPOINT_PLANNING_BOOTSTRAP_SNAPSHOT_REQUEST_V1_TYPE,
    NODE_ENDPOINT_PLANNING_BOOTSTRAP_SNAPSHOT_REQUEST_V1_SCHEMA,
    5,
    validate_snapshot_request
);

define_planning_bootstrap_message!(
    NodeEndpointPlanningBootstrapSnapshotObservedV1,
    NodeEndpointPlanningBootstrapSnapshotObservedV1Fields,
    observed: ComputePluginInstallPlanPlanningSnapshotObservedV2,
    NODE_ENDPOINT_PLANNING_BOOTSTRAP_SNAPSHOT_OBSERVED_V1_TYPE,
    NODE_ENDPOINT_PLANNING_BOOTSTRAP_SNAPSHOT_OBSERVED_V1_SCHEMA,
    6,
    validate_snapshot_observed
);

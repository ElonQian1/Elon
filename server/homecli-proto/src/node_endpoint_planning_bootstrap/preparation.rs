use crate::{
    ComputePluginInstallPlanPreparationObservedV1, ComputePluginInstallPlanPreparationRequestV1,
};

use super::{
    validation::{validate_preparation_observed, validate_preparation_request},
    NODE_ENDPOINT_PLANNING_BOOTSTRAP_PREPARATION_OBSERVED_V1_SCHEMA,
    NODE_ENDPOINT_PLANNING_BOOTSTRAP_PREPARATION_OBSERVED_V1_TYPE,
    NODE_ENDPOINT_PLANNING_BOOTSTRAP_PREPARATION_REQUEST_V1_SCHEMA,
    NODE_ENDPOINT_PLANNING_BOOTSTRAP_PREPARATION_REQUEST_V1_TYPE,
};

define_planning_bootstrap_message!(
    NodeEndpointPlanningBootstrapPreparationRequestV1,
    NodeEndpointPlanningBootstrapPreparationRequestV1Fields,
    request: ComputePluginInstallPlanPreparationRequestV1,
    NODE_ENDPOINT_PLANNING_BOOTSTRAP_PREPARATION_REQUEST_V1_TYPE,
    NODE_ENDPOINT_PLANNING_BOOTSTRAP_PREPARATION_REQUEST_V1_SCHEMA,
    3,
    validate_preparation_request
);

define_planning_bootstrap_message!(
    NodeEndpointPlanningBootstrapPreparationObservedV1,
    NodeEndpointPlanningBootstrapPreparationObservedV1Fields,
    observed: ComputePluginInstallPlanPreparationObservedV1,
    NODE_ENDPOINT_PLANNING_BOOTSTRAP_PREPARATION_OBSERVED_V1_TYPE,
    NODE_ENDPOINT_PLANNING_BOOTSTRAP_PREPARATION_OBSERVED_V1_SCHEMA,
    4,
    validate_preparation_observed
);

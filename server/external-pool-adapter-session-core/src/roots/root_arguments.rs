#[derive(Clone)]
pub struct ExternalPoolAdapterSessionRootArguments {
    pub(super) values: ExternalPoolAdapterSessionRootArgumentValues,
}

#[derive(Clone)]
pub(super) enum ExternalPoolAdapterSessionRootArgumentValues {
    Production([String; 6]),
    RuntimeCompatibility([String; 11]),
    TaskProtocolConformance([String; 14]),
    TaskProtocolProduction([String; 8]),
}

impl ExternalPoolAdapterSessionRootArguments {
    pub fn values(&self) -> &[String] {
        match &self.values {
            ExternalPoolAdapterSessionRootArgumentValues::Production(values) => values,
            ExternalPoolAdapterSessionRootArgumentValues::RuntimeCompatibility(values) => values,
            ExternalPoolAdapterSessionRootArgumentValues::TaskProtocolConformance(values) => values,
            ExternalPoolAdapterSessionRootArgumentValues::TaskProtocolProduction(values) => values,
        }
    }

    pub fn runtime_compatibility_values(&self) -> Option<&[String; 11]> {
        match &self.values {
            ExternalPoolAdapterSessionRootArgumentValues::Production(_) => None,
            ExternalPoolAdapterSessionRootArgumentValues::RuntimeCompatibility(values) => {
                Some(values)
            }
            ExternalPoolAdapterSessionRootArgumentValues::TaskProtocolConformance(_) => None,
            ExternalPoolAdapterSessionRootArgumentValues::TaskProtocolProduction(_) => None,
        }
    }

    pub fn task_protocol_conformance_values(&self) -> Option<&[String; 14]> {
        match &self.values {
            ExternalPoolAdapterSessionRootArgumentValues::TaskProtocolConformance(values) => {
                Some(values)
            }
            ExternalPoolAdapterSessionRootArgumentValues::Production(_)
            | ExternalPoolAdapterSessionRootArgumentValues::RuntimeCompatibility(_)
            | ExternalPoolAdapterSessionRootArgumentValues::TaskProtocolProduction(_) => None,
        }
    }

    pub fn task_protocol_production_values(&self) -> Option<&[String; 8]> {
        match &self.values {
            ExternalPoolAdapterSessionRootArgumentValues::TaskProtocolProduction(values) => {
                Some(values)
            }
            ExternalPoolAdapterSessionRootArgumentValues::Production(_)
            | ExternalPoolAdapterSessionRootArgumentValues::RuntimeCompatibility(_)
            | ExternalPoolAdapterSessionRootArgumentValues::TaskProtocolConformance(_) => None,
        }
    }

    #[cfg(feature = "test-support")]
    pub fn replace_for_test(&mut self, index: usize, value: String) {
        match &mut self.values {
            ExternalPoolAdapterSessionRootArgumentValues::Production(values) => {
                values[index] = value;
            }
            ExternalPoolAdapterSessionRootArgumentValues::RuntimeCompatibility(values) => {
                values[index] = value;
            }
            ExternalPoolAdapterSessionRootArgumentValues::TaskProtocolConformance(values) => {
                values[index] = value;
            }
            ExternalPoolAdapterSessionRootArgumentValues::TaskProtocolProduction(values) => {
                values[index] = value;
            }
        }
    }
}

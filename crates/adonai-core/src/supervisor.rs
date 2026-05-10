use serde::{Deserialize, Serialize};

use crate::{
    EndpointPolicy, EngineProbe, HardwareProfile, engines::probe_engines, hardware::scan_hardware,
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SupervisorState {
    Starting,
    Ready,
    Degraded,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SupervisorSnapshot {
    pub product: String,
    pub state: SupervisorState,
    pub version: String,
    pub endpoint_policy: EndpointPolicy,
    pub hardware: HardwareProfile,
    pub engines: EngineProbe,
}

impl SupervisorSnapshot {
    pub fn collect(endpoint_policy: EndpointPolicy) -> Self {
        Self {
            product: "Adonai".to_owned(),
            state: SupervisorState::Ready,
            version: env!("CARGO_PKG_VERSION").to_owned(),
            endpoint_policy,
            hardware: scan_hardware(),
            engines: probe_engines(),
        }
    }
}

#![forbid(unsafe_code)]

use xmip_core::{ArtifactId, ClusterId, NodeId};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum OperationalState {
    Green,
    Yellow,
    Red,
    Grey,
    Black,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Observation {
    pub state: OperationalState,
    pub artifact_id: Option<ArtifactId>,
    pub node_id: Option<NodeId>,
    pub cluster_id: Option<ClusterId>,
    pub summary: String,
    pub timestamp_unix_nanos: i128,
}

pub trait ObservationSink: Send + Sync {
    fn publish(&self, observation: Observation);
}

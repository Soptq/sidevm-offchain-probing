use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct ProbeParameters {
    pub dim_size: u64,
    pub sample_size: u64,
    pub detection_size: u64,
    pub batch_size: u64,
    pub beta: f64,
    pub lr: f64,
    pub patience: u64,
    pub factor: f64,
    pub min_lr: f64,
    pub max_iters: u64,
    pub max_offline_cnt: u8,

    pub eps: f64,
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct ProbeStatus {
    pub is_optimizing: bool,
    pub precision_ms: f64,
    pub epoch: u64,
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct HostMessage {
    pub command: String,
    pub data: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct QueryMessage {
    pub command: String,
    pub data: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct QueryEstimateRequest {
    pub from: String,
    pub to: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct QueryConnectedRequest {
    pub from: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct QueryBestEndpointRequest {
    pub to: String,
}

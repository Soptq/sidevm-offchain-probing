use serde::{Serialize, Deserialize};

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

    pub eps: f64,
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct ProbeStatus {
    pub precision_ms: f64,
}
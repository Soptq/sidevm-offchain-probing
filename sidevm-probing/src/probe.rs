use anyhow::{Result, anyhow};
use log::info;
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

use crate::types::{ProbeParameters, ProbeStatus};
use crate::utils::{cache_get, euclidean_distance, gen_random_vec, get_address_by_id, http_get};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Peer {
    pub encoded_public_key: String,
    pub best_endpoint: String,
    pub endpoints: Vec<String>,
    pub offline_cnt: u8,
}

impl Peer {
    pub async fn new(encoded_public_key: String) -> Result<Self> {
        let endpoints = get_address_by_id(&encoded_public_key).await?;
        Ok(Peer {
            encoded_public_key,
            best_endpoint: endpoints[0].clone(),
            endpoints,
            offline_cnt: 0,
        })
    }

    pub async fn update_endpoints(&mut self) {
        let endpoints = get_address_by_id(&self.encoded_public_key).await.unwrap();
        self.endpoints = endpoints;
    }

    pub async fn echo(&mut self) -> Result<f64> {
        info!("Echo to peer {}", &self.encoded_public_key);
        let start = SystemTime::now();
        let start_since_the_epoch = start
            .duration_since(UNIX_EPOCH)
            .expect("Time went backwards");

        let mut best_latency = u128::MAX;
        let mut all_endpoints_failed = true;

        for endpoint in &self.endpoints {
            let start_ms = start_since_the_epoch.as_millis();

            let url = format!("http://{}/echo/{}", endpoint, &start_ms);
            match http_get(&url).await {
                Ok(_) => {
                    all_endpoints_failed = false;
                }
                Err(_) => {
                    continue;
                }
            }

            let end = SystemTime::now();
            let end_since_the_epoch = end.duration_since(UNIX_EPOCH).expect("Time went backwards");
            let end_ms = end_since_the_epoch.as_millis();

            if end_ms - start_ms < best_latency {
                self.best_endpoint = endpoint.clone();
                best_latency = end_ms - start_ms;
            }
        }

        if all_endpoints_failed {
            return Err(anyhow!("All endpoints failed"));
        }

        // TODO: remove delay
        Ok((best_latency + 100) as f64)
    }

    pub async fn resolved(&self) -> Result<HashMap<String, Vec<f64>>> {
        info!("Fetch resolved data from peer {}", &self.encoded_public_key);
        let url = format!("http://{}/resolved", &self.best_endpoint);
        let response = http_get(&url).await?;
        let text = String::from_utf8(response).expect("Resolved data should be parseable");
        let resolved: HashMap<String, Vec<f64>> = serde_json::from_str(&text)?;

        Ok(resolved)
    }

    pub async fn notify_connected(&self, encoded_public_key: String) -> Result<()> {
        info!("Notify connected to peer {} from {}", &self.encoded_public_key, &encoded_public_key);
        let url = format!("http://{}/connected/{}", &self.best_endpoint, &encoded_public_key);
        http_get(&url).await?;

        Ok(())
    }

    pub fn is_online(&self) -> bool {
        self.offline_cnt == 0
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Probe {
    // identity
    pub encoded_public_key: String,
    // params
    pub parameters: ProbeParameters,
    // storages
    pub telemetry: HashMap<String, f64>,
    pub resolved: HashMap<String, Vec<f64>>,
    pub peers: HashMap<String, Peer>,
    pub pending_peer_ids: Vec<String>,
    // runtime status
    pub status: ProbeStatus,
}

impl Probe {
    pub fn new(public_key: Vec<u8>) -> Probe {
        let encoded_public_key = hex::encode(public_key);

        // get parameters from cache
        let dim_size = cache_get::<u64>(b"sidevm_probing::param::dim_size").unwrap_or(3 as u64);
        let sample_size =
            cache_get::<u64>(b"sidevm_probing::param::sample_size").unwrap_or(10 as u64);
        let detection_size =
            cache_get::<u64>(b"sidevm_probing::param::detection_size").unwrap_or(5 as u64);
        let batch_size =
            cache_get::<u64>(b"sidevm_probing::param::batch_size").unwrap_or(64 as u64);

        let beta = cache_get::<u64>(b"sidevm_probing::param::beta").unwrap_or(9 * 1e5 as u64)
            as f64
            / 1e6 as f64;

        let lr = cache_get::<u64>(b"sidevm_probing::param::lr").unwrap_or(1 * 1e6 as u64) as f64
            / 1e6 as f64;
        let patience = cache_get::<u64>(b"sidevm_probing::param::patience").unwrap_or(1000 as u64);
        let factor = cache_get::<u64>(b"sidevm_probing::param::factor").unwrap_or(1 * 1e5 as u64)
            as f64
            / 1e6 as f64;
        let min_lr = cache_get::<u64>(b"sidevm_probing::param::min_lr").unwrap_or(1 * 1e3 as u64)
            as f64
            / 1e6 as f64;
        let max_iters =
            cache_get::<u64>(b"sidevm_probing::param::max_iters").unwrap_or(10000 as u64);
        let max_offline_cnt =
            cache_get::<u8>(b"sidevm_probing::param::max_offline_cnt").unwrap_or(16 as u8);

        // initialize local database
        let mut telemetry = HashMap::new();
        let mut resolved = HashMap::new();

        telemetry.insert(encoded_public_key.clone(), 0 as f64);
        resolved.insert(
            encoded_public_key.clone(),
            gen_random_vec::<f64>(dim_size as usize),
        );

        // sidevm::ocall::local_cache_set(b"sidevm_probing::telemetry", &serde_json::to_string(&telemetry).unwrap().as_bytes()).unwrap();
        // sidevm::ocall::local_cache_set(b"sidevm_probing::resolve", &resolved.encode()).unwrap();
        // sidevm::ocall::local_cache_set(b"sidevm_probing::momentum", &momentum.encode()).unwrap();

        info!("Configuration for the probe:");
        info!("\t public key: {:?}", encoded_public_key);
        info!("\t dim size: {:?}", dim_size);
        info!("\t sample size: {:?}", sample_size);
        info!("\t detection size: {:?}", detection_size);
        info!("\t batch size: {:?}", batch_size);
        info!("\t beta: {:?}", beta);
        info!("\t lr: {:?}", lr);
        info!("\t patience: {:?}", patience);
        info!("\t factor: {:?}", factor);
        info!("\t min lr: {:?}", min_lr);
        info!("\t max iters: {:?}", max_iters);
        info!("\t max offline cnt: {:?}", max_offline_cnt);

        Probe {
            encoded_public_key,
            parameters: ProbeParameters {
                dim_size,
                sample_size,
                detection_size,
                batch_size,
                beta,
                lr,
                patience,
                factor,
                min_lr,
                max_iters,
                max_offline_cnt,
                eps: 1e-6 as f64,
            },
            telemetry,
            resolved,
            peers: HashMap::new(),
            pending_peer_ids: Vec::new(),
            status: ProbeStatus {
                is_optimizing: false,
                precision_ms: 0.0,
                epoch: 0,
            },
        }
    }

    pub async fn add_peer(&mut self, peer: Peer) -> Result<bool> {
        // check if the peer is already in the list
        if peer.encoded_public_key != self.encoded_public_key && !self.peers.contains_key(&peer.encoded_public_key) {
            self.peers.insert(peer.encoded_public_key.clone(), peer);
            return Ok(true);
        }

        Ok(false)
    }

    pub fn add_pending_peer(&mut self, encoded_public_key: String) {
        // check if the peer is already in the list
        if encoded_public_key != self.encoded_public_key && !self.peers.contains_key(&encoded_public_key) && !self.pending_peer_ids.contains(&encoded_public_key) {
            self.pending_peer_ids.push(encoded_public_key);
        }
    }

    pub fn get_best_endpoint_to(&self, encoded_public_key_to: String) -> Result<String> {
        if let Some(peer_to) = self.peers.get(&encoded_public_key_to) {
            if !peer_to.is_online() {
                return Err(anyhow!("Peer {} is offline", &encoded_public_key_to));
            }

            Ok(peer_to.best_endpoint.clone())
        } else {
            return Err(anyhow!("Peer {} is not in the list", &encoded_public_key_to));
        }
    }

    pub fn estimate(&self, encoded_public_key_from: String, encoded_public_key_to: String) -> Result<f64> {
        // ensure both of them are online
        if let Some(peer_from) = self.peers.get(&encoded_public_key_from) {
            if !peer_from.is_online() {
                return Err(anyhow!("Peer {} is offline", &encoded_public_key_from));
            }
        } else {
            return Err(anyhow!("Peer {} is not in the list", &encoded_public_key_from));
        }

        if let Some(peer_to) = self.peers.get(&encoded_public_key_to) {
            if !peer_to.is_online() {
                return Err(anyhow!("Peer {} is offline", &encoded_public_key_to));
            }
        } else {
            return Err(anyhow!("Peer {} is not in the list", &encoded_public_key_to));
        }

        let resolved_peer_from = self.resolved.get(&encoded_public_key_from)
            .ok_or(anyhow!("Peer {} is not resolved", &encoded_public_key_from))?;
        let resolved_peer_to = self.resolved.get(&encoded_public_key_to)
            .ok_or(anyhow!("Peer {} is not resolved", &encoded_public_key_to))?;

        Ok(euclidean_distance(&resolved_peer_from, &resolved_peer_to))
    }

    pub fn start_optimize(&mut self) {
        self.status.is_optimizing = true;
    }

    pub fn stop_optimize(&mut self) {
        self.status.is_optimizing = false;
    }
}

use anyhow::Result;
use log::{error, info};
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

use scale::Encode;
use serde::{Serialize, Deserialize};

use crate::utils::{cache_get, http_get, euclidean_distance, gen_random_vec};

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

    pub eps: f64,
}


#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Peer {
    pub encoded_public_key: String,
    pub host: String,
    pub port: u16,
}

impl Peer {
    pub fn new(encoded_public_key: String, host: String, port: u16) -> Self {
        Peer {
            encoded_public_key,
            host,
            port,
        }
    }

    pub async fn echo(&self) -> Result<f64> {
        info!("Echo to peer {}", &self.encoded_public_key);
        let start = SystemTime::now();
        let start_since_the_epoch = start
            .duration_since(UNIX_EPOCH)
            .expect("Time went backwards");
        let start_ms = start_since_the_epoch.as_millis();
        let url = format!("http://{}:{}/echo/{}", self.host, self.port, &start_ms);
        http_get(&url).await?;

        let end = SystemTime::now();
        let end_since_the_epoch = end
            .duration_since(UNIX_EPOCH)
            .expect("Time went backwards");
        let end_ms = end_since_the_epoch.as_millis();

        // TODO: remove delay
        Ok((end_ms - start_ms + 100) as f64)
    }

    pub async fn resolved(&self) -> Result<HashMap<String, Vec<f64>>> {
        info!("Fetch resolved data from peer {}", &self.encoded_public_key);
        let url = format!("http://{}:{}/resolved", &self.host, &self.port);
        let response = http_get(&url).await?;
        let text = String::from_utf8(response).expect("Resolved data should be parseable");
        let resolved: HashMap<String, Vec<f64>> = serde_json::from_str(&text)?;

        Ok(resolved)
    }

    pub async fn peers(&self) -> Result<HashMap<String, Peer>> {
        info!("Fetch peer data from peer {}", &self.encoded_public_key);
        let url = format!("http://{}:{}/peers", &self.host, &self.port);
        let response = http_get(&url).await?;
        let text = String::from_utf8(response).expect("Peer data should be parseable");
        let peers: HashMap<String, Peer> = serde_json::from_str(&text)?;

        Ok(peers)
    }

    pub async fn add_peer(&self, encoded_public_key: &str, host: &str, port: u16) -> Result<()> {
        info!("Add peer {}:{}[{}] to peer {}", &host, &port, &encoded_public_key, &self.encoded_public_key);
        let url = format!("http://{}:{}/add_peer/{}/{}/{}", &self.host, &self.port, &encoded_public_key, &host, &port);
        let response = http_get(&url).await?;

        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct Probe {
    // identity
    pub encoded_public_key: String,
    // params
    pub parameters: ProbeParameters,
    // storages
    pub telemetry: HashMap<String, f64>,
    pub resolved: HashMap<String, Vec<f64>>,
    pub peers: HashMap<String, Peer>,
}

impl Probe {
    pub fn new(public_key: Vec<u8>) -> Probe {
        let encoded_public_key = hex::encode(public_key);

        // get parameters from cache
        let dim_size = cache_get::<u64>(b"sidevm_probing::param::dim_size")
            .unwrap_or(3 as u64);
        let sample_size = cache_get::<u64>(b"sidevm_probing::param::sample_size")
            .unwrap_or(10 as u64);
        let detection_size = cache_get::<u64>(b"sidevm_probing::param::detection_size")
            .unwrap_or(5 as u64);
        let batch_size = cache_get::<u64>(b"sidevm_probing::param::batch_size")
            .unwrap_or(64 as u64);

        let beta = cache_get::<u64>(b"sidevm_probing::param::beta")
            .unwrap_or(9 * 1e5 as u64) as f64 / 1e6 as f64;

        let lr = cache_get::<u64>(b"sidevm_probing::param::lr")
            .unwrap_or(1 * 1e6 as u64) as f64 / 1e6 as f64;
        let patience = cache_get::<u64>(b"sidevm_probing::param::patience")
            .unwrap_or(1000 as u64);
        let factor = cache_get::<u64>(b"sidevm_probing::param::factor")
            .unwrap_or(1 * 1e5 as u64) as f64 / 1e6 as f64;
        let min_lr = cache_get::<u64>(b"sidevm_probing::param::min_lr")
            .unwrap_or(1 * 1e3 as u64) as f64 / 1e6 as f64;

        // initialize local database
        let mut telemetry = HashMap::new();
        let mut resolved = HashMap::new();
        let mut peers = HashMap::new();

        telemetry.insert(encoded_public_key.clone(), 0 as f64);
        resolved.insert(encoded_public_key.clone(), gen_random_vec::<f64>(dim_size as usize));

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
                eps: 1e-6 as f64,
            },
            telemetry,
            resolved,
            peers,
        }
    }

    pub fn add_peer(&mut self, encoded_public_key: String, host: String, port: u16) -> Result<()> {
        let peer = Peer::new(encoded_public_key.clone(), host, port);
        // check if the peer is already in the list
        if self.peers.contains_key(&encoded_public_key) {
            info!("Peer {} is already in the peer list", &encoded_public_key);
            return Ok(());
        }

        // initialize peer position
        if let None = self.resolved.get_mut(&peer.encoded_public_key) {
            self.resolved.insert(peer.encoded_public_key.clone(), gen_random_vec::<f64>(self.parameters.dim_size as usize));
        }

        self.peers.insert(peer.encoded_public_key.clone(), peer);

        Ok(())
    }

    pub fn estimate(&self, encoded_public_key: String) -> Result<f64> {
        let peer_position = match self.resolved.get(&encoded_public_key) {
            Some(value) => value,
            None => {
                info!("Peer {} is not in the resolved list", encoded_public_key);
                return Ok(0.0 as f64);
            },
        };

        let my_position = self.resolved.get(&self.encoded_public_key).expect("My position should be found");

        Ok(euclidean_distance(&my_position, &peer_position))
    }
}
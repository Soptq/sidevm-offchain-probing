use anyhow::Result;
use log::{error, info};
use std::collections::HashMap;
use std::convert::Infallible;
use std::time::Duration;

use rand::{seq::IteratorRandom, thread_rng};

use probe::{Peer, Probe};
use router::router;
use service::RouterService;
use types::{ProbeParameters, ProbeStatus};
use utils::{euclidean_distance, gen_random_vec};

use std::sync::Arc;
use tokio::sync::Mutex;

mod probe;
mod router;
mod service;
mod types;
mod utils;

pub type AppState = Arc<Mutex<Option<Probe>>>;

const PHANTOM_BREAK: Duration = Duration::from_millis(0);
const TIMEOUT_MAX_MILLIS: f64 = 1.0 * 60.0 * 1000.0; // 1 minute

async fn init_pink_input() -> Result<(), Infallible> {
    info!("Initializing pink input...");
    loop {
        if let Some(message) = sidevm::channel::input_messages().next().await {
            let msg = String::from_utf8_lossy(&message);
            info!("Received message: {}", msg);
            match msg.as_ref() {
                "init_params" => {
                    info!("Initializing parameters...");
                    // let public_key = cache_get::<Vec<u8>>(b"sidevm_probing::param::public_key")
                    //     .expect("failed to get public key");
                    // if let None = probe {
                    //     probe = Some(Probe::new());
                    // } else {
                    //     error!("Probe already initialized");
                    // }
                }
                "start_probing" => {
                    info!("Starting probing...");
                }
                "stop_probing" => {
                    info!("Stop probing...");
                }
                "purge_cache" => {
                    info!("Purge Cache...");
                }
                _ => {
                    info!("Unknown message: {}", msg);
                }
            }
        } else {
            info!("Input message channel closed");
        }
    }

    Ok(())
}

async fn init_server(address: &str, app_state: AppState) -> Result<()> {
    let router = router(app_state);
    let service = RouterService::new(router).expect("failed to create service");

    info!("Listening on {}", address);

    let listener = sidevm::net::TcpListener::bind(address).await?;

    let server = hyper::Server::builder(listener.into_addr_incoming())
        .executor(sidevm::exec::HyperExecutor)
        .serve(service);
    if let Err(e) = server.await {
        error!("server error: {}", e);
    }

    Ok(())
}

async fn optimize(app_state: AppState, host: &str, port: u16) -> Result<()> {
    loop {
        let mut encoded_public_key: String = String::default();
        let mut parameters: ProbeParameters = ProbeParameters::default();
        let mut telemetry: HashMap<String, f64> = HashMap::new();
        let mut resolved: HashMap<String, Vec<f64>> = HashMap::new();
        let mut status: ProbeStatus = ProbeStatus::default();

        let mut peers: HashMap<String, Peer> = HashMap::new();
        let mut possible_peers: Vec<Peer> = Vec::new();

        // clone a copy of necessary data
        {
            let mut lock = app_state.lock().await;
            let probe = (*lock).as_ref().expect("should be able to get probe ref");
            encoded_public_key = probe.encoded_public_key.clone();
            parameters = probe.parameters.clone();
            telemetry = probe.telemetry.clone();
            resolved = probe.resolved.clone();
            peers = probe.peers.clone();
        }

        sidevm::time::sleep(PHANTOM_BREAK).await;

        // collect telemetry
        {
            let mut rng = thread_rng();
            let batch_peers_id = peers
                .keys()
                .cloned()
                .choose_multiple(&mut rng, parameters.detection_size as usize);
            for peer_id in &batch_peers_id {
                sidevm::time::sleep(PHANTOM_BREAK).await;
                let peer = peers.get(peer_id).expect("peer should be in the peers");
                // send "friend" request
                peer.add_peer(&encoded_public_key, host, port).await;

                // collect ttl
                let ttl = match peer.echo().await {
                    Ok(ttl) => ttl,
                    Err(_) => TIMEOUT_MAX_MILLIS,
                };

                if let Some(value) = telemetry.get_mut(&peer.encoded_public_key) {
                    *value = *value * parameters.beta + ttl * (1.0 - parameters.beta);
                } else {
                    telemetry.insert(peer.encoded_public_key.clone(), ttl);
                }

                // update peers
                let external_peers = match peer.peers().await {
                    Ok(peers) => peers,
                    Err(_) => HashMap::new(),
                };

                for ext_peer in external_peers.values() {
                    possible_peers.push(ext_peer.clone());
                }
                info!("Peers discovery: {:?}", &possible_peers);
            }
        }

        sidevm::time::sleep(PHANTOM_BREAK).await;

        // start optimizing
        {
            let mut my_position: Vec<f64> = resolved
                .get(&encoded_public_key)
                .expect(format!("{} should be in the resolved data", &encoded_public_key).as_str())
                .to_vec();
            let mut momentum: Vec<f64> = vec![0.0 as f64; parameters.dim_size as usize];
            let mut min_loss: f64 = f64::MAX;
            let mut current_lr: f64 = parameters.lr;

            let mut iteration: u64 = 0;
            let mut patience: u64 = 0;

            loop {
                // if it reaches the maximum number of iterations, stop optimizing
                if iteration >= parameters.max_iters {
                    break;
                }
                // early return if learning rate reaches threshold
                if &current_lr < &parameters.min_lr {
                    break;
                }
                iteration += 1;
                // step 1: random sample a batch of telemetry data to process
                let mut rng = thread_rng();
                let batch_peers_id = peers
                    .keys()
                    .cloned()
                    .choose_multiple(&mut rng, parameters.batch_size as usize);
                // step 2: local optimize
                let mut force: Vec<f64> = vec![0.0 as f64; parameters.dim_size as usize];
                let mut peers_len: usize = 0;
                for peer_id in &batch_peers_id {
                    sidevm::time::sleep(PHANTOM_BREAK).await;
                    let peer = peers.get(peer_id).expect("peer should be in the peers");
                    if !telemetry.contains_key(&peer.encoded_public_key) {
                        continue;
                    }
                    peers_len += 1;

                    let ground_truth = telemetry.get(&peer.encoded_public_key).expect(
                        format!(
                            "{} should be in the telemetry data",
                            &peer.encoded_public_key
                        )
                        .as_str(),
                    );

                    if !resolved.contains_key(&peer.encoded_public_key) {
                        resolved.insert(
                            peer.encoded_public_key.clone(),
                            gen_random_vec::<f64>(parameters.dim_size as usize),
                        );
                    }
                    let peer_position = resolved.get(&peer.encoded_public_key).expect(
                        format!(
                            "{} should be in the resolved data",
                            &peer.encoded_public_key
                        )
                        .as_str(),
                    );

                    let prediction = euclidean_distance(&my_position, &peer_position);
                    let error = ground_truth - prediction;
                    let direction = my_position
                        .iter()
                        .zip(peer_position.iter())
                        .map(|(i, j)| i - j)
                        .collect::<Vec<f64>>();
                    // normalize the direction and get force
                    let norm = direction.iter().fold(0.0, |acc, x| acc + x.powi(2));
                    force = force
                        .iter()
                        .zip(direction.iter())
                        .map(|(f, x)| f + (x / (norm.sqrt() + parameters.eps)) * error)
                        .collect::<Vec<f64>>();
                }
                if peers_len == 0 {
                    break;
                }
                // step 3: update position
                // update momentum
                momentum = momentum
                    .iter()
                    .zip(force.iter())
                    .map(|(i, j)| {
                        i * parameters.beta + j * (1.0 - parameters.beta) / peers_len as f64
                    })
                    .collect::<Vec<f64>>();
                // update my position
                my_position = my_position
                    .iter()
                    .zip(momentum.iter())
                    .map(|(i, j)| i + j * current_lr)
                    .collect::<Vec<f64>>();
                // step 4: calculate loss and update parameters
                let mut test_total_loss: f64 = 0.0;
                for (test_entry, test_label) in telemetry.iter() {
                    sidevm::time::sleep(PHANTOM_BREAK).await;
                    if test_entry == &encoded_public_key {
                        // skip my own data
                        continue;
                    }
                    let test_peer_position = resolved
                        .get(test_entry)
                        .expect(format!("{} should be in the resolved data", test_entry).as_str());
                    let test_prediction = euclidean_distance(&my_position, &test_peer_position);
                    let test_error = (test_label - test_prediction).abs();
                    test_total_loss += test_error / (telemetry.len() as f64 - 1.0 + parameters.eps);
                }
                if test_total_loss < min_loss {
                    min_loss = test_total_loss;
                    patience = 0;
                } else {
                    patience += 1;
                }
                if patience > parameters.patience {
                    current_lr *= parameters.factor;
                    patience = 0;
                }
                if iteration % 1000 == 0 {
                    info!(
                        "Iteration: {}, Loss: {}, Min Loss {}, Learning Rate: {}",
                        iteration, test_total_loss, min_loss, current_lr
                    );
                }
            }

            resolved.insert(encoded_public_key.clone(), my_position);
            status.precision_ms = min_loss;
        }

        sidevm::time::sleep(PHANTOM_BREAK).await;

        // Aggregate from other peers' resolved.
        // TODO: Momentum accelerated aggregation
        {
            let mut rng = thread_rng();
            let batch_peers_id = peers
                .keys()
                .cloned()
                .choose_multiple(&mut rng, parameters.sample_size as usize);
            let mut aggregation_counter = HashMap::<String, u64>::new();
            for peer_id in &batch_peers_id {
                let peer = peers.get(peer_id).expect("peer should be in the peers");
                let peer_resolved = match peer.resolved().await {
                    Ok(resolved) => resolved,
                    Err(_) => continue,
                };
                for (k, v) in peer_resolved {
                    sidevm::time::sleep(PHANTOM_BREAK).await;
                    if let Some(value) = resolved.get_mut(&k) {
                        *value = (*value
                            .iter()
                            .zip(v.iter())
                            .map(|(i, j)| i + j)
                            .collect::<Vec<f64>>())
                        .to_vec();
                        if let Some(value) = aggregation_counter.get_mut(&k) {
                            *value += 1;
                        } else {
                            aggregation_counter.insert(k.clone(), 2);
                        }
                    } else {
                        resolved.insert(k.clone(), v);
                        aggregation_counter.insert(k.clone(), 1);
                    }
                }
            }
            for (k, v) in &aggregation_counter {
                let value = resolved.get_mut(k).expect("should be in the resolved data");
                *value = (value
                    .iter()
                    .map(|i| i / v.clone() as f64)
                    .collect::<Vec<f64>>())
                .to_vec();
            }
            // rebase resolved data so that the center of all positions is at the origin
            if aggregation_counter.len() > 0 {
                let center = resolved.values().fold(
                    vec![0.0 as f64; parameters.dim_size as usize],
                    |acc, x| {
                        acc.iter()
                            .zip(x.iter())
                            .map(|(i, j)| i + j / resolved.len() as f64)
                            .collect::<Vec<f64>>()
                    },
                );
                resolved = resolved
                    .iter()
                    .map(|(k, v)| {
                        (
                            k.clone(),
                            v.iter()
                                .zip(center.iter())
                                .map(|(i, j)| i - j)
                                .collect::<Vec<f64>>(),
                        )
                    })
                    .collect::<HashMap<String, Vec<f64>>>();
            }
        }

        sidevm::time::sleep(PHANTOM_BREAK).await;

        // update the app_state
        {
            let mut lock = app_state.lock().await;
            let mut probe = (*lock).as_mut().expect("should be able to get mut ref");
            probe.telemetry = telemetry;
            probe.resolved = resolved;
            probe.status = status;
            for possible_peer in &possible_peers {
                sidevm::time::sleep(PHANTOM_BREAK).await;
                probe.add_peer(
                    possible_peer.encoded_public_key.clone(),
                    possible_peer.host.clone(),
                    possible_peer.port,
                );
            }
            possible_peers.clear();
        }

        sidevm::time::sleep(Duration::from_secs(5)).await;
    }

    Ok(())
}

#[sidevm::main]
async fn main() {
    sidevm::logger::Logger::with_max_level(log::Level::Trace).init();
    sidevm::ocall::enable_ocall_trace(true).unwrap();

    // TODO
    let worker_id: u16 = 5;
    let host = "127.0.0.1";
    let port: u16 = 2000 + worker_id;
    let address = format!("{}:{}", host, port);
    let test_public_key: &[u8] = &[0u8, 0u8, 0u8, worker_id as u8];
    let app_state = Arc::new(Mutex::new(Some(Probe::new(test_public_key.to_vec()))));

    tokio::select! {
        _ = init_pink_input() => {},
        _ = init_server(&address, Arc::clone(&app_state)) => {},
        _ = optimize(Arc::clone(&app_state), host, port) => {},
    }
}

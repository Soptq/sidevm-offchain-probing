use anyhow::{anyhow, Result};
use log::{info, warn};
use std::collections::HashMap;
use std::time::Duration;

use rand::{seq::IteratorRandom, thread_rng};

use crate::probe::Peer;
use crate::utils::{euclidean_distance, gen_random_vec};
use crate::types::{ProbeParameters, ProbeStatus};
use crate::AppState;


async fn compute_loss(
    encoded_public_key: &String,
    peers: &HashMap<String, Peer>,
    telemetry: &HashMap<String, f64>,
    resolved: &HashMap<String, Vec<f64>>,
    eps: f64) -> Result<f64>
{
    let my_position: Vec<f64> = resolved
        .get(encoded_public_key)
        .expect(format!("{} should be in the resolved data", encoded_public_key).as_str())
        .to_vec();
    let mut test_total_loss: f64 = 0.0;
    for (test_entry, test_label) in telemetry.iter() {
        if test_entry == encoded_public_key {
            continue;
        }
        if let Some(peer) = peers.get(test_entry) {
            if !peer.is_online() {
                continue;
            }
        }
        let test_peer_position = resolved
            .get(test_entry)
            .expect(format!("{} should be in the resolved data", test_entry).as_str());
        let test_prediction = euclidean_distance(&my_position, &test_peer_position);
        let test_error = (test_label - test_prediction).abs();
        test_total_loss += test_error / (telemetry.len() as f64 - 1.0 + eps);
        sidevm::time::maybe_rest().await;
    }

    Ok(test_total_loss)
}

async fn collect_telemetry(
    telemetry: &mut HashMap<String, f64>,
    peers: &mut HashMap<String, Peer>,
    batch_peers_id: &Vec<String>,
    beta: f64,
) -> Result<()> {
    for peer_id in batch_peers_id {
        let mut peer = peers.get_mut(peer_id)
            .ok_or(anyhow!("{} should be in the peers data", peer_id))?;

        peer.update_endpoints().await;
        // collect ttl
        match peer.echo().await {
            Ok(ttl) => {
                peer.offline_cnt = 0;
                if let Some(value) = telemetry.get_mut(&peer.encoded_public_key) {
                    *value = *value * beta + ttl * (1.0 - beta);
                } else {
                    telemetry.insert(peer.encoded_public_key.clone(), ttl);
                }
            },
            Err(_) => {
                peer.offline_cnt += 1;
            },
        };
        sidevm::time::maybe_rest().await;
    }

    Ok(())
}

pub async fn optimize(app_state: AppState) -> Result<()> {
    loop {
        let mut encoded_public_key: String = String::default();
        let mut parameters: ProbeParameters = ProbeParameters::default();
        let mut telemetry: HashMap<String, f64> = HashMap::new();
        let mut resolved: HashMap<String, Vec<f64>> = HashMap::new();
        let mut status: ProbeStatus = ProbeStatus::default();

        let mut peers: HashMap<String, Peer> = HashMap::new();
        let mut pending_peer_ids: Vec<String> = Vec::new();

        // clone a copy of necessary data
        {
            let lock = app_state.lock().await;
            let probe = (*lock).as_ref().expect("should be able to get probe ref");
            encoded_public_key = probe.encoded_public_key.clone();
            parameters = probe.parameters.clone();
            telemetry = probe.telemetry.clone();
            resolved = probe.resolved.clone();
            peers = probe.peers.clone();
            status = probe.status.clone();
        }

        if !status.is_optimizing {
            sidevm::time::sleep(Duration::from_secs(10)).await;
            continue;
        }
        sidevm::time::maybe_rest().await;

        // collect telemetry
        // here we will independently collect telemetry from 5 online peers and 5 offline peers
        {
            let mut online_peers = peers.clone();
            online_peers.retain(|_, peer| peer.is_online());
            let mut offline_peers = peers.clone();
            offline_peers.retain(|_, peer| !peer.is_online());

            let mut rng = thread_rng();
            let online_batch_peers_id = online_peers
                .keys()
                .cloned()
                .choose_multiple(&mut rng, parameters.detection_size as usize);
            let offline_batch_peers_id = offline_peers
                .keys()
                .cloned()
                .choose_multiple(&mut rng, parameters.detection_size as usize);

            collect_telemetry(&mut telemetry, &mut peers, &online_batch_peers_id, parameters.beta).await?;
            collect_telemetry(&mut telemetry, &mut peers, &offline_batch_peers_id, parameters.beta).await?;
        }
        let mut retained_peers = peers.clone();
        retained_peers.retain(|_, peer| peer.is_online());

        sidevm::time::maybe_rest().await;

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
                // here we will not choose peers that are offline
                let batch_peers_id = retained_peers
                    .keys()
                    .cloned()
                    .choose_multiple(&mut rng, parameters.batch_size as usize);
                // step 2: local optimize
                let mut force: Vec<f64> = vec![0.0 as f64; parameters.dim_size as usize];
                let mut peers_len: usize = 0;
                for peer_id in &batch_peers_id {
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
                    sidevm::time::maybe_rest().await;
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
                let test_total_loss = compute_loss(&encoded_public_key, &retained_peers, &telemetry, &resolved, parameters.eps).await?;
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
        }

        sidevm::time::maybe_rest().await;

        // Aggregate from other peers' resolved.
        {
            let mut rng = thread_rng();
            // here we will not choose peers that are offline
            let batch_peers_id = retained_peers
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
                    // update peers
                    if !pending_peer_ids.contains(&k) {
                        pending_peer_ids.push(k.clone());
                    }
                    // update model
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
                    sidevm::time::maybe_rest().await;
                }
                info!("Peers discovery: {:?}", &pending_peer_ids);
            }
            for (k, v) in &aggregation_counter {
                let value = resolved.get_mut(k).expect("should be in the resolved data");
                *value = (value
                    .iter()
                    .map(|i| i / v.clone() as f64)
                    .collect::<Vec<f64>>())
                    .to_vec();
                sidevm::time::maybe_rest().await;
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

        status.precision_ms = compute_loss(&encoded_public_key, &retained_peers, &telemetry, &resolved, parameters.eps).await?;
        status.epoch = (status.epoch + 1) % u64::MAX;

        sidevm::time::maybe_rest().await;

        // update the app_state
        let mut peers_to_notify = Vec::new();
        {
            let mut lock = app_state.lock().await;
            let mut probe = (*lock).as_mut().expect("should be able to get mut ref");
            probe.telemetry = telemetry;
            probe.resolved = resolved;
            probe.peers = peers;
            probe.pending_peer_ids.extend(pending_peer_ids);
            probe.status = status;

            // add pending peers
            for pending_peer_id in probe.pending_peer_ids.clone() {
                let peer = Peer::new(pending_peer_id.clone()).await?;
                let added = probe.add_peer(peer.clone()).await?;
                if added {
                    peers_to_notify.push(peer);
                }
            }
            probe.pending_peer_ids.clear();
            // remove offline peers where its `offline_cnt` reaches threshold.
            probe.peers.retain(|_, peer| peer.offline_cnt < parameters.max_offline_cnt);
        }

        for peer in peers_to_notify {
            peer.notify_connected(encoded_public_key.clone())
                .await
                .map_err(|err| warn!("Failed to notify {} about the connection: {:?}", &peer.encoded_public_key, err))
                .ok();
        }

        sidevm::time::sleep(Duration::from_secs(5)).await;
    }
}
use anyhow::{Result};
use log::{info};

use crate::AppState;
use crate::types;

pub async fn init_pink_query(app_state: AppState) -> Result<()> {
    info!("Initializing pink query...");
    loop {
        if let Some(query) = sidevm::channel::incoming_queries().next().await {
            let payload_str = String::from_utf8_lossy(&query.payload);
            let msg: types::QueryMessage = serde_json::from_str(&payload_str)?;
            info!("Received host query: {:?} from: {:?}", msg, query.origin);
            match msg.command.as_str() {
                "echo" => {
                    let _ = query.reply_tx.send(msg.data.as_bytes());
                }
                "resolved" => {
                    let lock = app_state.lock().await;
                    let probe = (*lock).as_ref().unwrap();

                    let resolved = serde_json::to_string(&probe.resolved).unwrap();
                    let _ = query.reply_tx.send(resolved.as_bytes());
                }
                "estimate" => {
                    let estimate_request: types::QueryEstimateRequest = serde_json::from_str(&msg.data)?;
                    let peer_id_from = estimate_request.from;
                    let peer_id_to = estimate_request.to;

                    let lock = app_state.lock().await;
                    let probe = (*lock).as_ref().unwrap();
                    let estimation = probe.estimate(peer_id_from.clone(), peer_id_to.clone())
                        .unwrap_or(-1.0 as f64);

                    let _ = query.reply_tx.send(estimation.to_string().as_bytes());
                }
                "connected" => {
                    let connected_request: types::QueryConnectedRequest = serde_json::from_str(&msg.data)?;
                    let peer_id = connected_request.from;

                    let mut lock = app_state.lock().await;
                    let probe = (*lock).as_mut().unwrap();
                    probe.add_pending_peer(peer_id.clone());

                    let _ = query.reply_tx.send(peer_id.clone().as_bytes());
                }
                "best_endpoint" => {
                    let best_endpoint_request: types::QueryBestEndpointRequest = serde_json::from_str(&msg.data)?;
                    let peer_id = best_endpoint_request.to;

                    let lock = app_state.lock().await;
                    let probe = (*lock).as_ref().unwrap();
                    let best_endpoint = probe.get_best_endpoint_to(peer_id.clone()).unwrap();

                    let _ = query.reply_tx.send(best_endpoint.as_bytes());
                }
                "status" => {
                    let lock = app_state.lock().await;
                    let probe = (*lock).as_ref().unwrap();

                    let status = serde_json::to_string(&probe.status).unwrap();
                    let _ = query.reply_tx.send(status.as_bytes());
                }
                _ => {
                    info!("Unknown message: {:?}", msg);
                }
            }
        } else {
            info!("Query channel closed");
        }
    }

    // Unreachable code
}
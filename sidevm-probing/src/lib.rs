use anyhow::{anyhow, Result};
use log::{error, info};

use probe::Probe;
use router::router;
use service::RouterService;
use optimize::optimize;
use query::init_pink_query;
use utils::get_address_by_id;

use std::sync::Arc;
use tokio::sync::Mutex;

mod probe;
mod router;
mod query;
mod service;
mod optimize;
mod types;
mod utils;

pub type AppState = Arc<Mutex<Option<Probe>>>;

async fn init_pink_input(app_state: AppState) -> Result<()> {
    info!("Initializing pink input...");
    loop {
        if let Some(message) = sidevm::channel::input_messages().next().await {
            let message_str = String::from_utf8_lossy(&message);
            let msg: types::HostMessage = serde_json::from_str(&message_str)?;
            info!("Received host message: {:?}", msg);
            match msg.command.as_str() {
                "add_peer" => {
                    let mut lock = app_state.lock().await;
                    let probe = (*lock).as_mut().expect("should be able to get probe ref");
                    probe.add_pending_peer(msg.data.clone());
                }
                "start_optimize" => {
                    let mut lock = app_state.lock().await;
                    let probe = (*lock).as_mut().expect("should be able to get probe ref");
                    probe.start_optimize();
                }
                "stop_optimize" => {
                    let mut lock = app_state.lock().await;
                    let probe = (*lock).as_mut().expect("should be able to get probe ref");
                    probe.stop_optimize();
                }
                "save_app" => {
                    let lock = app_state.lock().await;
                    let probe = (*lock).as_ref().expect("should be able to get probe ref");
                    let probe_state = serde_json::to_string(&probe)?;
                    sidevm::ocall::local_cache_set(b"sidevm_probing::probe_state", &probe_state.as_bytes())
                        .expect("should be able to set local cache");
                }
                "load_app" => {
                    let probe_state = sidevm::ocall::local_cache_get(b"sidevm_probing::probe_state")?
                        .ok_or(anyhow!("Probe state not found in local cache"))?;
                    let restored_probe: Probe = serde_json::from_str(&String::from_utf8_lossy(&probe_state))?;
                    let mut lock = app_state.lock().await;
                    *lock = Some(restored_probe);
                }
                _ => {
                    info!("Unknown message: {:?}", msg);
                }
            }
        } else {
            info!("Input message channel closed");
        }
    }

    // Unreachable code
}

async fn init_server(address: &str, app_state: AppState) -> Result<()> {
    let router = router(app_state);
    let service = RouterService::new(router).expect("failed to create service");

    let listener = sidevm::net::TcpListener::bind(address).await?;

    info!("Listening on {}", address);

    let server = hyper::Server::builder(listener.into_addr_incoming())
        .executor(sidevm::exec::HyperExecutor)
        .serve(service);
    if let Err(e) = server.await {
        error!("server error: {}", e);
    }

    Ok(())
}

#[sidevm::main]
async fn main() {
    sidevm::logger::Logger::with_max_level(log::Level::Trace).init();
    sidevm::ocall::enable_ocall_trace(true).unwrap();

    let mut worker_id: u16 = 0;
    if let Some(message) = sidevm::channel::input_messages().next().await {
        let message_str = String::from_utf8_lossy(&message);
        info!("Received host message: {:?}", message_str);
        worker_id = message_str.parse::<u16>().unwrap();
    }

    // TODO
    let test_public_key: &[u8] = &[0u8, 0u8, 0u8, worker_id as u8];
    let endpoints = get_address_by_id(&hex::encode(test_public_key.clone())).await.unwrap();
    let address = endpoints[0].clone();
    let app_state = Arc::new(Mutex::new(Some(Probe::new(test_public_key.to_vec()))));

    tokio::select! {
        _ = init_pink_input(Arc::clone(&app_state)) => {},
        _ = init_pink_query(Arc::clone(&app_state)) => {},
        _ = init_server(&address, Arc::clone(&app_state)) => {},
        _ = optimize(Arc::clone(&app_state)) => {},
    }
}

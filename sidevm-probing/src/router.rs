use std::convert::Infallible;
use log::{error, info};

use hyper::{Request, Response, Body};

use routerify::prelude::*;
use routerify::Router;

use tokio::sync::Mutex;
use std::sync::Arc;

use crate::{Probe, AppState};

async fn root_handler(req: Request<Body>) -> Result<Response<Body>, Infallible> {
    log::info!("accessing root handler");
    Ok(Response::new(Body::from("Welcome to the Phala's off-chain probing service\n")))
}

async fn echo_handler(req: Request<Body>) -> Result<Response<Body>, Infallible> {
    let msg = req.param("msg").unwrap();
    Ok(Response::new(Body::from(msg.clone())))
}

async fn telemetry_handler(req: Request<Body>) -> Result<Response<Body>, Infallible> {
    log::info!("telemetry");
    let state = req.data::<AppState>().unwrap();
    let lock = state.lock().await;
    let probe = (*lock).as_ref().unwrap();

    let telemetry = serde_json::to_string(&probe.telemetry).unwrap();
    Ok(Response::new(Body::from(telemetry)))
}

async fn resolved_handler(req: Request<Body>) -> Result<Response<Body>, Infallible> {
    log::info!("resolved");
    let state = req.data::<AppState>().unwrap();
    let lock = state.lock().await;
    let probe = (*lock).as_ref().unwrap();

    let resolved = serde_json::to_string(&probe.resolved).unwrap();
    Ok(Response::new(Body::from(resolved)))
}

async fn peers_handler(req: Request<Body>) -> Result<Response<Body>, Infallible> {
    log::info!("peers");
    let state = req.data::<AppState>().unwrap();
    let lock = state.lock().await;
    let probe = (*lock).as_ref().unwrap();

    let peers = serde_json::to_string(&probe.peers).unwrap();
    Ok(Response::new(Body::from(peers)))
}

async fn add_peer_handler(req: Request<Body>) -> Result<Response<Body>, Infallible> {
    let peer_id = req.param("peer_id").unwrap();
    let host = req.param("host").unwrap();
    let port = req.param("port").unwrap().parse::<u16>().unwrap();
    let state = req.data::<AppState>().unwrap();
    let mut lock = state.lock().await;
    let mut probe = (*lock).as_mut().unwrap();
    probe.add_peer(peer_id.clone(), host.clone(), port.clone());

    Ok(Response::new(Body::from("add_peer\n")))
}

async fn estimate_handler(req: Request<Body>) -> Result<Response<Body>, Infallible> {
    let peer_id = req.param("peer_id").unwrap();
    let state = req.data::<AppState>().unwrap();
    let mut lock = state.lock().await;
    let mut probe = (*lock).as_mut().unwrap();
    let estimation = probe.estimate(peer_id.clone()).unwrap();

    Ok(Response::new(Body::from(format!("{}\n", estimation))))
}

pub fn router(app_state: AppState) -> Router<Body, Infallible> {
    Router::builder()
        .data(app_state)
        .get("/", root_handler)
        .get("/echo/:msg", echo_handler)
        .get("/telemetry", telemetry_handler)
        .get("/resolved", resolved_handler)
        .get("/peers", peers_handler)
        .get("/add_peer/:peer_id/:host/:port", add_peer_handler)
        .get("/estimate/:peer_id", estimate_handler)
        .build()
        .unwrap()
}
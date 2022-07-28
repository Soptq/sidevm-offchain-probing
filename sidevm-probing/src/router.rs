use log::{error, info};
use std::convert::Infallible;

use hyper::{Body, Request, Response};

use routerify::prelude::*;
use routerify::Router;

use crate::AppState;

async fn echo_handler(req: Request<Body>) -> Result<Response<Body>, Infallible> {
    info!("GET /echo/:msg");
    let msg = req.param("msg").unwrap();
    Ok(Response::new(Body::from(msg.clone())))
}

async fn telemetry_handler(req: Request<Body>) -> Result<Response<Body>, Infallible> {
    info!("GET /telemetry");
    let state = req.data::<AppState>().unwrap();
    let lock = state.lock().await;
    let probe = (*lock).as_ref().unwrap();

    let telemetry = serde_json::to_string(&probe.telemetry).unwrap();
    Ok(Response::new(Body::from(telemetry)))
}

async fn resolved_handler(req: Request<Body>) -> Result<Response<Body>, Infallible> {
    info!("GET /resolved");
    let state = req.data::<AppState>().unwrap();
    let lock = state.lock().await;
    let probe = (*lock).as_ref().unwrap();

    let resolved = serde_json::to_string(&probe.resolved).unwrap();
    Ok(Response::new(Body::from(resolved)))
}

async fn estimate_handler(req: Request<Body>) -> Result<Response<Body>, Infallible> {
    info!("GET /estimate/:from/:to");
    let peer_id_from = req.param("from").unwrap();
    let peer_id_to = req.param("to").unwrap();
    let state = req.data::<AppState>().unwrap();
    let lock = state.lock().await;
    let probe = (*lock).as_ref().unwrap();
    let estimation = probe.estimate(peer_id_from.clone(), peer_id_to.clone())
        .unwrap_or(-1.0 as f64);

    Ok(Response::new(Body::from(format!("{}\n", estimation))))
}

async fn connected_handler(req: Request<Body>) -> Result<Response<Body>, Infallible> {
    info!("GET /connected/:from");
    let peer_id = req.param("from").unwrap();
    let state = req.data::<AppState>().unwrap();
    let mut lock = state.lock().await;
    let probe = (*lock).as_mut().unwrap();

    probe.add_pending_peer(peer_id.clone()).await;
    Ok(Response::new(Body::from(peer_id.clone())))
}

async fn status_handler(req: Request<Body>) -> Result<Response<Body>, Infallible> {
    info!("GET /status");
    let state = req.data::<AppState>().unwrap();
    let lock = state.lock().await;
    let probe = (*lock).as_ref().unwrap();

    let status = serde_json::to_string(&probe.status).unwrap();
    Ok(Response::new(Body::from(status)))
}

pub fn router(app_state: AppState) -> Router<Body, Infallible> {
    Router::builder()
        .data(app_state)
        .get("/echo/:msg", echo_handler)
        .get("/resolved", resolved_handler)
        .get("/estimate/:from/:to", estimate_handler)
        .get("/connected/:from", connected_handler)
        .get("/status", status_handler)
        .get("/debug/telemetry", telemetry_handler)
        .build()
        .unwrap()
}

use std::io::Read;
use anyhow::Result;
use log::{error, info};
use scale::Decode;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use sidevm::net::HttpConnector;
use hyper::body::Buf;
use rand::distributions::Standard;
use rand::prelude::Distribution;

pub fn cache_get<T>(key: &[u8]) -> Option<T>
    where T: Decode,
{
    if let Ok(Some(value)) = sidevm::ocall::local_cache_get(key) {
        return Some(T::decode(&mut &value[..]).expect("failed to decode"));
    }

    None
}

pub fn gen_random_vec<T: Default + Clone>(len: usize) -> Vec<T>
    where Standard: Distribution<T> {
    let mut vec: Vec<T> = vec![T::default(); len];
    for i in 0..len {
        vec[i] = rand::random::<T>();
    }
    vec
}

pub fn euclidean_distance(a: &[f64], b: &[f64]) -> f64 {
    let mut sum = 0.0;
    for (i, j) in a.iter().zip(b.iter()) {
        sum += (i - j).powi(2);
    }
    sum.sqrt()
}

pub async fn http_get(url: &str) -> Result<Vec<u8>> {
    info!("Connecting to {}", url);
    let connector = HttpConnector::new();
    let client = hyper::Client::builder()
        .executor(sidevm::exec::HyperExecutor)
        .build::<_, String>(connector);
    let response = client
        .get(url.parse().expect("Bad url"))
        .await?;
    info!("response status: {}", response.status());

    let mut buf = vec![];
    hyper::body::aggregate(response)
        .await?
        .reader()
        .read_to_end(&mut buf)?;

    Ok(buf)
}

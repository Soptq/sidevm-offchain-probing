use anyhow::Result;
use hyper::body::Buf;
use log::{error, info};
use rand::distributions::Standard;
use rand::prelude::Distribution;
use scale::Decode;
use sidevm::net::HttpConnector;
use std::io::Read;

pub fn cache_get<T>(key: &[u8]) -> Option<T>
where
    T: Decode,
{
    if let Ok(Some(value)) = sidevm::ocall::local_cache_get(key) {
        return Some(T::decode(&mut &value[..]).expect("failed to decode"));
    }

    None
}

pub fn gen_random_vec<T: Default + Clone>(len: usize) -> Vec<T>
where
    Standard: Distribution<T>,
{
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

pub async fn get_address_by_id(peer_id: &str) -> Result<(String, u16)> {
    let (host, port) = match peer_id {
        "00000000" => ("127.0.0.1".to_string(), 2000),
        "00000001" => ("127.0.0.1".to_string(), 2001),
        "00000002" => ("127.0.0.1".to_string(), 2002),
        "00000003" => ("127.0.0.1".to_string(), 2003),
        "00000004" => ("127.0.0.1".to_string(), 2004),
        "00000005" => ("127.0.0.1".to_string(), 2005),
        "00000006" => ("127.0.0.1".to_string(), 2006),
        "00000007" => ("127.0.0.1".to_string(), 2007),
        _ => panic!("Unknown peer id"),
    };

    Ok((host, port as u16))
}

pub async fn http_get(url: &str) -> Result<Vec<u8>> {
    info!("Connecting to {}", url);
    let connector = HttpConnector::new();
    let client = hyper::Client::builder()
        .executor(sidevm::exec::HyperExecutor)
        .build::<_, String>(connector);
    let response = client.get(url.parse().expect("Bad url")).await?;
    info!("response status: {}", response.status());

    let mut buf = vec![];
    hyper::body::aggregate(response)
        .await?
        .reader()
        .read_to_end(&mut buf)?;

    Ok(buf)
}

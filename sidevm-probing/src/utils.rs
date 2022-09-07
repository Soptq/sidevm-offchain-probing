use anyhow::Result;
use hyper::body::Buf;
use log::info;
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

// TODO: replace
pub async fn get_address_by_id(peer_id: &str) -> Result<Vec<String>> {
    let endpoints = match peer_id {
        "00000000" => vec!("127.0.0.1:2000".to_string()),
        "00000001" => vec!("127.0.0.1:2001".to_string()),
        "00000002" => vec!("127.0.0.1:2002".to_string()),
        "00000003" => vec!("127.0.0.1:2003".to_string()),
        "00000004" => vec!("127.0.0.1:2004".to_string()),
        "00000005" => vec!("127.0.0.1:2005".to_string()),
        "00000006" => vec!("127.0.0.1:2006".to_string()),
        "00000007" => vec!("127.0.0.1:2007".to_string()),
        _ => panic!("Unknown peer id"),
    };

    Ok(endpoints)
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

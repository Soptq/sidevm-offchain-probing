#![cfg_attr(not(feature = "std"), no_std)]

use pink_extension as pink;


#[pink::contract]
mod phala_probe {
    use scale::Encode;
    use super::pink;
    use pink::chain_extension::{signing::{derive_sr25519_key, get_public_key}, SigType};

    #[ink(storage)]
    pub struct PhalaProbe {}

    impl PhalaProbe {
        #[ink(constructor)]
        pub fn default() -> Self {
            let code = &include_bytes!("./sideprog.wasm")[..];
            pink::start_sidevm(code.into(), true);

            let private_key = derive_sr25519_key(b"phala-offline-probing");
            let public_key = get_public_key(&private_key, SigType::Sr25519);
            pink::ext().cache_set(b"sidevm_probing::param::public_key", &public_key.encode()).unwrap();

            pink::ext().cache_set(b"sidevm_probing::param::dim_size", &(3 as u64).encode()).unwrap();
            pink::ext().cache_set(b"sidevm_probing::param::sample_size", &(10 as u64).encode()).unwrap();
            pink::ext().cache_set(b"sidevm_probing::param::detection_size", &(5 as u64).encode()).unwrap();
            pink::ext().cache_set(b"sidevm_probing::param::batch_size", &(64 as u64).encode()).unwrap();

            pink::ext().cache_set(b"sidevm_probing::param::beta", &(9 * 1e5 as u64).encode()).unwrap();

            pink::ext().cache_set(b"sidevm_probing::param::lr", &(1 * 1e6 as u64).encode()).unwrap();
            pink::ext().cache_set(b"sidevm_probing::param::patience", &(1000 as u64).encode()).unwrap();
            pink::ext().cache_set(b"sidevm_probing::param::factor", &(1 * 1e5 as u64).encode()).unwrap();
            pink::ext().cache_set(b"sidevm_probing::param::min_lr", &(1 * 1e3 as u64).encode()).unwrap();

            pink::push_sidevm_message(b"init_params".to_vec());

            Self {}
        }
        #[ink(message)]
        pub fn test(&self) {
        }
    }
}

[]

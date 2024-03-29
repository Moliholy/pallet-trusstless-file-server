#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;
use alloc::string::String;
use sp_std::vec::Vec;

sp_api::decl_runtime_apis! {
    pub trait TrustlessFileServerApi {
        fn get_files() -> Vec<(Vec<u8>, u32)>;
        fn get_proof(merkle_root: Vec<u8>, position: u32) -> Option<(String, Vec<Vec<u8>>)>;
    }
}

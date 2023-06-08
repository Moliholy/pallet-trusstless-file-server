#![cfg_attr(not(feature = "std"), no_std)]

use alloc::borrow::ToOwned;
use alloc::format;
use frame_support::log;
use frame_support::sp_runtime::offchain::http;
use frame_support::sp_runtime::offchain::http::Request;
use sp_core::offchain::Duration;
use sp_std::vec;
use sp_std::vec::Vec;

const BOUNDARY: &[u8] = b"------BOUNDARY";

pub fn ipfs_get_hash_from_sha256(hash: &[u8; 32]) -> Vec<u8> {
    let full_data: Vec<_> = vec![vec![0x12, 0x20], hash.to_vec()]
        .into_iter()
        .flatten()
        .collect();
    bs58::encode(full_data).into_vec()
}

fn make_multipart(data: &[u8]) -> Vec<u8> {
    BOUNDARY
        .iter()
        .chain(b"\nContent-Disposition: form-data; name=\"file\"\n\n")
        .chain(data)
        .chain(b"\n")
        .chain(BOUNDARY)
        .chain(b"--")
        .copied()
        .collect::<Vec<u8>>()
}

pub fn ipfs_upload(base_url: &str, data: &[u8]) -> Result<(), http::Error> {
    let url = base_url.to_owned() + "/api/v0/block/put";
    let multipart = make_multipart(data);
    let header =
        format!("multipart/form-data; boundary={}", core::str::from_utf8(BOUNDARY).unwrap());
    let request = Request::post(&url, vec![multipart.as_slice()]);
    let deadline = sp_io::offchain::timestamp().add(Duration::from_millis(5_000));
    let pending = request
        .add_header("Content-Type", header.as_str())
        .deadline(deadline)
        .send()
        .map_err(|_| http::Error::IoError)?;
    let response = pending
        .try_wait(deadline)
        .map_err(|_| http::Error::DeadlineReached)??;
    if response.code == 200 {
        log::info!("Chunk successfully uploaded");
    } else {
        log::warn!("Unexpected status code: {}", response.code);
        return Err(http::Error::Unknown);
    };
    Ok(())
}

#[cfg(feature = "std")]
#[cfg(test)]
mod test {
    use sp_io::hashing::sha2_256;

    use super::*;

    #[test]
    fn test_ipfs_hash_works() {
        let content = b"hello world".as_slice();
        let hash = sha2_256(content);
        assert_eq!(
            ipfs_get_hash_from_sha256(&hash),
            b"QmaozNR7DZHQK1ZcU9p7QdrshMvXqWK6gpu5rmrkPdT3L4"
        );
    }
}

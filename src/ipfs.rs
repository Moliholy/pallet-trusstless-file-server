#![cfg_attr(not(feature = "std"), no_std)]

use alloc::borrow::ToOwned;
use alloc::format;
use alloc::string::String;
use frame_support::log;
use frame_support::sp_runtime::offchain::http;
use frame_support::sp_runtime::offchain::http::Request;
use sp_std::vec;
use sp_std::vec::Vec;

const BOUNDARY: &[u8] = b"------BOUNDARY";

pub fn ipfs_get_hash_from_sha256(hash: &[u8; 32]) -> String {
    let full_data: Vec<_> = vec![vec![0x12, 0x20], hash.to_vec()]
        .into_iter()
        .flatten()
        .collect();
    bs58::encode(full_data).into_string()
}

fn make_multipart(data: &[u8]) -> Vec<u8> {
    BOUNDARY
        .iter()
        .chain(b"\r\nContent-Disposition: form-data; name=\"file\"\r\nContent-Type: application/octet-stream\r\n\r\n")
        .chain(data)
        .chain(b"\r\n--")
        .chain(BOUNDARY)
        .chain(b"--\r\n")
        .copied()
        .collect::<Vec<u8>>()
}

pub fn ipfs_upload(base_url: &str, data: &[u8]) -> Result<(), http::Error> {
    let url = base_url.to_owned() + "/api/v0/block/put";
    let multipart = make_multipart(data);
    let request = Request::post(&url, vec![multipart.as_slice()]).add_header(
        "Content-Type",
        format!("multipart/form-data; boundary={}", core::str::from_utf8(BOUNDARY).unwrap())
            .as_str(),
    );
    let pending = request.send().map_err(|_| http::Error::IoError)?;
    let response = pending.wait()?;
    if response.code == 200 {
        log::info!("Chunk successfully uploaded");
    } else {
        let body = response.body().collect::<Vec<u8>>();
        log::warn!(
            "Unexpected status code: {}.\n{}",
            response.code,
            core::str::from_utf8(&body).unwrap()
        );
        return Err(http::Error::Unknown);
    };
    Ok(())
}

#[cfg(test)]
mod test {
    use sp_io::hashing::sha2_256;

    use super::*;

    #[test]
    fn test_ipfs_hash_works() {
        let content = b"hello world".as_slice();
        let hash = sha2_256(content);
        assert_eq!(
            ipfs_get_hash_from_sha256(&hash).as_str(),
            "QmaozNR7DZHQK1ZcU9p7QdrshMvXqWK6gpu5rmrkPdT3L4"
        );
    }
}

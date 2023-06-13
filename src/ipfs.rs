use alloc::borrow::ToOwned;
use alloc::format;
use alloc::string::String;

use binascii::b32encode;
use frame_support::log;
use frame_support::sp_runtime::offchain::http;
use frame_support::sp_runtime::offchain::http::Request;
use sp_std::vec;
use sp_std::vec::Vec;

const BOUNDARY: &[u8] = b"------BOUNDARY";

pub fn ipfs_get_hash_from_sha256(hash: &[u8; 32]) -> String {
    // CIDv1, raw binary (multicodec), sha2 (hash), digest length (32 bytes)
    let extra_bytes = vec![0x01, 0x55, 0x12, 0x20];
    let full_data: Vec<_> = vec![extra_bytes, hash.to_vec()]
        .into_iter()
        .flatten()
        .collect();
    let mut buff = [0u8; 256];
    let bytes = b32encode(full_data.as_slice(), &mut buff).unwrap();
    ("b".to_owned() + core::str::from_utf8(bytes).unwrap())
        // remove right equal signs
        .trim_end_matches('=')
        // remove capitals
        .to_lowercase()
}

fn make_multipart(data: &[u8]) -> Vec<u8> {
    b"--"
        .iter()
        .chain(BOUNDARY)
        .chain(b"\r\nContent-Disposition: form-data; name=\"file\"\r\nContent-Type: application/octet-stream\r\n\r\n")
        .chain(data)
        .chain(b"\r\n--")
        .chain(BOUNDARY)
        .chain(b"--\r\n")
        .copied()
        .collect()
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
    let response_body = response.body();
    let raw_body = response_body.collect::<Vec<u8>>();
    let body = core::str::from_utf8(&raw_body).unwrap();
    if response.code == 200 {
        log::info!("Chunk successfully uploaded: {}", body);
    } else {
        log::warn!("Unexpected status code: {}.\n{}", response.code, body);
        return Err(http::Error::Unknown);
    };
    Ok(())
}

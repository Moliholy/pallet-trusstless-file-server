use std::sync::Arc;

use jsonrpsee::{
    core::{Error as JsonRpseeError, RpcResult},
    proc_macros::rpc,
    types::error::{CallError, ErrorObject},
};
use sp_api::ProvideRuntimeApi;
use sp_blockchain::HeaderBackend;
use sp_runtime::generic::BlockId;
use sp_runtime::traits::Block as BlockT;

pub use pallet_trustless_file_server_runtime_api::TrustlessFileServerApi as TrustlessFileServerRuntimeApi;

#[derive(serde::Deserialize, serde::Serialize)]
pub struct HashItem {
    hash: String,
    pieces: u32,
}

#[derive(serde::Deserialize, serde::Serialize)]
pub struct MerkleProof {
    content: String,
    proof: Vec<String>,
}

#[rpc(client, server)]
pub trait TrustlessFileServerApi<BlockHash> {
    #[method(name = "trustless_file_server_get_files")]
    fn get_files(&self, at: Option<BlockHash>) -> RpcResult<Vec<HashItem>>;

    #[method(name = "trustless_file_server_get_proof")]
    fn get_proof(
        &self,
        at: Option<BlockHash>,
        merkle_root: String,
        position: u32,
    ) -> RpcResult<MerkleProof>;
}

/// A struct that implements the `TrustlessFileServerApi`.
pub struct TrustlessFileServerPallet<C, Block> {
    // If you have more generics, no need to TrustlessFileServerPallet<C, M, N, P, ...>
    // just use a tuple like TrustlessFileServerPallet<C, (M, N, P, ...)>
    client: Arc<C>,
    _marker: std::marker::PhantomData<Block>,
}

impl<C, Block> TrustlessFileServerPallet<C, Block> {
    /// Create new `TrustlessFileServerPallet` instance with the given reference to the client.
    pub fn new(client: Arc<C>) -> Self {
        Self {
            client,
            _marker: Default::default(),
        }
    }
}

impl<C, Block> TrustlessFileServerApiServer<<Block as BlockT>::Hash>
    for TrustlessFileServerPallet<C, Block>
where
    Block: BlockT,
    C: ProvideRuntimeApi<Block> + HeaderBackend<Block> + Send + Sync + 'static,
    C::Api: TrustlessFileServerRuntimeApi<Block>,
{
    fn get_files(&self, at: Option<<Block as BlockT>::Hash>) -> RpcResult<Vec<HashItem>> {
        let api = self.client.runtime_api();
        let at = BlockId::hash(at.unwrap_or_else(|| self.client.info().best_hash));

        let result = api.get_files(at).map_err(runtime_error_into_rpc_err)?;
        let hashes = result
            .into_iter()
            .map(|item| HashItem {
                pieces: item.1,
                hash: vec_to_hex_string(&item.0),
            })
            .collect();
        Ok(hashes)
    }

    fn get_proof(
        &self,
        at: Option<<Block as BlockT>::Hash>,
        merkle_root: String,
        position: u32,
    ) -> RpcResult<MerkleProof> {
        let api = self.client.runtime_api();
        let at = BlockId::hash(at.unwrap_or_else(|| self.client.info().best_hash));
        let merkle_root_bytes = array_bytes::hex2bytes(merkle_root)
            .map_err(runtime_error_into_rpc_err)?
            .to_vec();
        let result = api
            .get_proof(at, merkle_root_bytes, position)
            .map_err(runtime_error_into_rpc_err)?;
        match result {
            Some((content, proof)) => Ok(MerkleProof {
                content: vec_to_hex_string(&content),
                proof: proof.iter().map(|hash| vec_to_hex_string(hash)).collect(),
            }),
            None => Err(runtime_error_into_rpc_err("Failure getting the merkle proof")),
        }
    }
}

const RUNTIME_ERROR: i32 = 1;

fn vec_to_hex_string(data: &[u8]) -> String {
    data.iter()
        .map(|b| format!("{:02x}", b))
        .collect::<Vec<String>>()
        .join("")
}

/// Converts a runtime trap into an RPC error.
fn runtime_error_into_rpc_err(err: impl std::fmt::Debug) -> JsonRpseeError {
    CallError::Custom(ErrorObject::owned(
        RUNTIME_ERROR,
        "Runtime error",
        Some(format!("{:?}", err)),
    ))
    .into()
}

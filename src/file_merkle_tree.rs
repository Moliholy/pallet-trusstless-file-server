use core::mem;

use codec::{Decode, Encode, EncodeLike, MaxEncodedLen};
use frame_support::pallet_prelude::ConstU32;
use frame_support::BoundedVec;
use scale_info::build::Fields;
use scale_info::{Path, Type, TypeInfo};
use sp_io::hashing::sha2_256;
use sp_std::vec;
use sp_std::vec::Vec;

/// File chunks to build the merkle tree are hardcoded to 1KB
const DEFAULT_CHUNK_SIZE: usize = 1024;
/// Length of a sha256 hash, in bytes.
const HASH_SIZE: usize = 32;
/// Maximum number of pieces the merkle tree can have
const MAX_MERKLE_TREE_NODES: u32 = 64;
/// Maximum size of the merkle tree
const MAX_MERKLE_TREE_SIZE: u32 = MAX_MERKLE_TREE_NODES * HASH_SIZE as u32;
/// In case the number of bytes is not a power of two, we fill with zeroes.
const CHUNK_FILLER: [u8; 32] = [0u8; 32];

fn calculate_chunk_size(file_size: usize) -> usize {
    let mut chunk_size = file_size / 64;
    if chunk_size < DEFAULT_CHUNK_SIZE {
        // minimum chunk size is 1KB
        chunk_size = DEFAULT_CHUNK_SIZE;
    }
    chunk_size
}

fn calculate_has_boundary(file_size: usize) -> bool {
    file_size % calculate_chunk_size(file_size) != 0
}

fn calculate_pieces(file_size: usize) -> u32 {
    let chunk_size = calculate_chunk_size(file_size);
    let mut pieces = file_size / chunk_size;
    if calculate_has_boundary(file_size) {
        pieces += 1;
    }
    pieces as u32
}

/// Represents the data structure of a merkle tree.
/// It includes also the raw file content.
#[derive(Default, Clone, PartialEq)]
pub struct FileMerkleTree {
    pub merkle_tree: BoundedVec<u8, ConstU32<MAX_MERKLE_TREE_SIZE>>,
    pub file_size: usize,
    pub boundary_hash: Option<BoundedVec<u8, ConstU32<32>>>,
}

impl MaxEncodedLen for FileMerkleTree {
    fn max_encoded_len() -> usize {
        mem::size_of::<usize>() + BoundedVec::<u8, ConstU32<64>>::max_encoded_len()
    }
}

impl Encode for FileMerkleTree {
    fn encode(&self) -> Vec<u8> {
        let file_size = self.file_size.to_le_bytes();
        let mut result = Vec::from(file_size.as_slice());
        if let Some(boundary) = &self.boundary_hash {
            result.extend_from_slice(boundary.as_slice());
        }
        result.extend_from_slice(&self.merkle_tree);
        result
    }
}

impl Decode for FileMerkleTree {
    fn decode<I: codec::Input>(input: &mut I) -> Result<Self, codec::Error> {
        let mut buff = [0u8; 4];
        input.read(&mut buff)?;
        let file_size = u32::from_le_bytes(buff);
        let boundary_hash = if calculate_has_boundary(file_size as usize) {
            let mut bytes = vec![0u8; HASH_SIZE];
            input.read(&mut bytes).unwrap();
            Some(bytes.try_into().unwrap())
        } else {
            None
        };
        input.read(&mut buff)?;
        let merkle_tree_len = input.remaining_len()?.unwrap();
        let mut bytes = vec![0u8; merkle_tree_len];
        input.read(&mut bytes)?;
        Ok(FileMerkleTree {
            file_size: file_size as usize,
            merkle_tree: bytes.try_into().unwrap(),
            boundary_hash,
        })
    }
}

impl TypeInfo for FileMerkleTree {
    type Identity = Self;

    fn type_info() -> Type {
        Type::builder()
            .path(Path::new("FileMerkleTree", module_path!()))
            .composite(
                Fields::named()
                    .field(|f| f.ty::<Vec<u8>>().name("file_bytes").type_name("Vec<u8>"))
                    .field(|f| f.ty::<Vec<u8>>().name("merkle_tree").type_name("Vec<u8>"))
                    .field(|f| f.ty::<u32>().name("pieces").type_name("u32")),
            )
    }
}

impl EncodeLike for FileMerkleTree {}

impl FileMerkleTree {
    /// Constructs a `FileMerkleTree` out of the provided file bytes.
    /// It builds the whole merkle tree and keeps file contents.
    pub fn new(file_bytes: &[u8]) -> Self {
        let chunk_size = calculate_chunk_size(file_bytes.len());
        let chunks = file_bytes.chunks(chunk_size);
        let pieces = chunks.len();
        let mut boundary_hash = None;
        let mut tree = chunks
            .map(|chunk| {
                if chunk.len() != chunk_size {
                    // process last chunk
                    boundary_hash = Some(sha2_256(chunk).to_vec().try_into().unwrap());
                    let mut result = vec![0u8; chunk_size];
                    for (index, byte) in chunk.iter().enumerate() {
                        result[index] = *byte;
                    }
                    sha2_256(result.as_slice())
                } else {
                    sha2_256(chunk)
                }
            })
            .fold(Vec::<u8>::new(), |mut acc, hash| {
                acc.append(&mut hash.to_vec());
                acc
            });
        // make the tree a totally balanced binary tree
        let mut num_items = pieces.next_power_of_two();
        for _ in 0..(num_items - pieces) {
            tree.extend_from_slice(&CHUNK_FILLER);
        }
        let mut pos = 0;
        while num_items > 1 {
            for i in (pos..(num_items + pos)).step_by(2) {
                let slice1 = &tree[(i * HASH_SIZE)..((i + 1) * HASH_SIZE)];
                let slice2 = &tree[((i + 1) * HASH_SIZE)..((i + 2) * HASH_SIZE)];
                let mut result = Vec::with_capacity(HASH_SIZE * 2);
                result.extend_from_slice(slice1);
                result.extend_from_slice(slice2);
                let hash = sha2_256(result.as_slice());
                tree.extend_from_slice(&hash);
            }
            pos += num_items;
            num_items /= 2;
        }
        Self {
            file_size: file_bytes.len(),
            merkle_tree: tree.try_into().unwrap(),
            boundary_hash,
        }
    }

    pub fn chunk_size(&self) -> usize {
        calculate_chunk_size(self.file_size)
    }

    pub fn pieces(&self) -> u32 {
        calculate_pieces(self.file_size)
    }

    pub fn file_chunk_hash_at(&self, position: u32) -> Option<[u8; HASH_SIZE]> {
        let pieces = self.pieces();
        if position >= pieces {
            return None;
        }
        if position == pieces - 1 {
            if let Some(boundary) = &self.boundary_hash {
                return Some(boundary[..].try_into().unwrap());
            }
        }
        let pos = position as usize * HASH_SIZE;
        let limit = pos + HASH_SIZE;
        Some(self.merkle_tree[pos..limit].try_into().unwrap())
    }

    /// Returns the merkle root of this file.
    /// The merkle root is stored as the last 32 bytes of the `merkle_tree` array.
    pub fn merkle_root(&self) -> &[u8] {
        &self.merkle_tree[self.merkle_tree.len() - HASH_SIZE..]
    }

    fn find_proof(
        &self,
        position: usize,
        first_index: usize,
        base: usize,
        proof: &mut Vec<Vec<u8>>,
    ) {
        if base == 1 {
            // we do not need to return the merkle root
            return;
        }
        let sibling = if position % 2 == 0 {
            position + 1
        } else {
            position - 1
        };
        let parent = (position - first_index) / 2 + first_index + base;
        let hash = self.merkle_tree[sibling * HASH_SIZE..((sibling + 1) * HASH_SIZE)].to_vec();
        proof.push(hash);
        self.find_proof(parent, first_index + base, base / 2, proof);
    }

    /// Finds the content and merkle proof of a given piece
    /// The piece is identified by its position.
    ///
    /// Returns a tuple with the given chunk content and the merkle proof.
    /// The sha256 of the content can be used to compute the merkle root hash
    /// along with the merkle proof.
    pub fn merkle_proof(&self, piece: u32) -> Option<Vec<Vec<u8>>> {
        if piece >= self.pieces() {
            return None;
        }
        let mut proof = Vec::new();
        self.find_proof(piece as usize, 0, self.pieces().next_power_of_two() as usize, &mut proof);
        Some(proof)
    }
}

#[cfg(test)]
mod test {
    use sp_io::hashing::sha2_256;

    use super::*;

    #[test]
    fn test_merkle_tree_should_work() {
        let content = include_bytes!("../img/substrate.png");
        let tree = FileMerkleTree::new(content);

        // check sizes
        let chunk_size = tree.chunk_size();
        assert_eq!(chunk_size, DEFAULT_CHUNK_SIZE);
        assert_eq!(tree.pieces(), 12);

        // check hashes
        for (index, chunk) in content.chunks(chunk_size).enumerate() {
            assert_eq!(tree.file_chunk_hash_at(index as u32), Some(sha2_256(chunk)));
        }
        assert_eq!(tree.file_chunk_hash_at(12), None);

        // check proof
        let merkle_root = tree.merkle_root();
        let proof = match tree.merkle_proof(0) {
            None => panic!("Could not get the proof"),
            Some(p) => p,
        };
        assert_eq!(proof.len(), 4);
        let first_chunk = content.chunks(chunk_size).next().unwrap();
        let mut current = sha2_256(first_chunk).to_vec();
        for hash in proof {
            current = sha2_256(&[current, hash].concat()).to_vec();
        }
        assert_eq!(current.as_slice(), merkle_root);
    }
}

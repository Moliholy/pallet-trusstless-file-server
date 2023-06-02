use codec::{Decode, Encode, EncodeLike};
use scale_info::{Path, Type, TypeInfo};
use scale_info::build::Fields;
use sp_io::hashing::sha2_256;
use sp_std::vec;
use sp_std::vec::Vec;

/// File chunks to build the merkle tree are hardcoded to 1KB
const CHUNK_SIZE: usize = 1024;
/// Length of a sha256 hash, in bytes.
const HASH_SIZE: usize = 32;
/// In case the number of bytes is not a power of two, we fill with zeroes.
const CHUNK_FILLER: [u8; 32] = [0u8; 32];


/// Represents the data structure of a merkle tree.
/// It includes also the raw file content.
#[derive(Default, Clone, PartialEq)]
pub struct FileMerkleTree {
    pub file_bytes: Vec<u8>,
    pub merkle_tree: Vec<u8>,
    pub pieces: u32,
}

impl Encode for FileMerkleTree {
    fn encode(&self) -> Vec<u8> {
        let file_size = (self.file_bytes.len() as u32).to_le_bytes();
        let mut result = Vec::from(file_size.as_slice());
        result.extend_from_slice(self.pieces.to_le_bytes().as_slice());
        result.extend_from_slice(&self.file_bytes);
        result.extend_from_slice(&self.merkle_tree);
        result
    }
}

impl Decode for FileMerkleTree {
    fn decode<I: codec::Input>(input: &mut I) -> Result<Self, codec::Error> {
        let mut buff = [0u8; 4];
        input.read(&mut buff)?;
        let file_size = u32::from_le_bytes(buff);
        input.read(&mut buff)?;
        let pieces = u32::from_le_bytes(buff);
        let mut file_bytes = vec![0u8; file_size as usize];
        input.read(&mut file_bytes)?;
        let merkle_tree_len = input.remaining_len()?.unwrap();
        let mut merkle_tree = vec![0u8; merkle_tree_len];
        input.read(&mut merkle_tree)?;
        Ok(FileMerkleTree { file_bytes, merkle_tree, pieces })
    }
}

impl TypeInfo for FileMerkleTree {
    type Identity = Self;

    fn type_info() -> Type {
        Type::builder().path(Path::new("FileMerkleTree", module_path!())).composite(
            Fields::named()
                .field(|f| {
                    f.ty::<Vec<u8>>()
                        .name("file_bytes")
                        .type_name("Vec<u8>")
                })
                .field(|f| {
                    f.ty::<Vec<u8>>()
                        .name("merkle_tree")
                        .type_name("Vec<u8>")
                })
                .field(|f| {
                    f.ty::<u32>()
                        .name("pieces")
                        .type_name("u32")
                })
        )
    }
}

impl EncodeLike for FileMerkleTree {}

impl FileMerkleTree {
    /// Constructs a `FileMerkleTree` out of the provided file bytes.
    /// It builds the whole merkle tree and keeps file contents.
    pub fn new(file_bytes: Vec<u8>) -> Self {
        let chunks = file_bytes.chunks(CHUNK_SIZE);
        let pieces = chunks.len();
        let mut tree = chunks.map(|chunk| {
            if chunk.len() != CHUNK_SIZE {
                // process last chunk
                let mut result = vec![0u8; CHUNK_SIZE];
                for (index, byte) in chunk.iter().enumerate() {
                    result[index] = *byte;
                }
                sha2_256(result.as_slice())
            } else {
                sha2_256(&chunk)
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
                let hash = sha2_256(&result.as_slice());
                tree.extend_from_slice(&hash);
            }
            pos += num_items;
            num_items /= 2;
        }
        Self {
            file_bytes,
            pieces: pieces as u32,
            merkle_tree: tree,
        }
    }

    fn file_chunk_at(&self, position: u32) -> &[u8] {
        let pos = position as usize * CHUNK_SIZE;
        let limit = if position == (self.pieces - 1) {
            self.file_bytes.len()
        } else { pos + CHUNK_SIZE };
        &self.file_bytes[pos..limit]
    }

    /// Returns the merkle root of this file.
    /// The merkle root is stored as the last 32 bytes of the `merkle_tree` array.
    pub fn merkle_root(&self) -> &[u8] {
        &self.merkle_tree[self.merkle_tree.len() - HASH_SIZE..]
    }

    fn find_proof(&self, position: usize, first_index: usize, base: usize, proof: &mut Vec<Vec<u8>>) {
        if base == 1 {
            // we do not need to return the merkle root
            return;
        }
        let sibling = if position % 2 == 0 { position + 1 } else { position - 1 };
        let parent = (position - first_index) / 2 + first_index + base;
        let hash = self.merkle_tree[sibling * HASH_SIZE..((sibling + 1) * HASH_SIZE)].to_vec();
        proof.push(hash);
        self.find_proof(parent, first_index + base,base / 2, proof);
    }

    /// Finds the content and merkle proof of a given piece
    /// The piece is identified by its position.
    ///
    /// Returns a tuple with the given chunk content and the merkle proof.
    /// The sha256 of the content can be used to compute the merkle root hash
    /// along with the merkle proof.
    pub fn merkle_proof(&self, piece: u32) -> Option<(Vec<u8>, Vec<Vec<u8>>)> {
        if piece >= self.pieces {
            return None;
        }
        let content = self.file_chunk_at(piece).to_vec();
        let proof: Vec<Vec<u8>> = if self.pieces == 1 {
            vec![self.file_chunk_at(piece).to_vec()]
        } else {
            let mut proof = Vec::new();
            self.find_proof(piece as usize, 0, self.pieces.next_power_of_two() as usize, &mut proof);
            proof
        };
        Some((content, proof))
    }
}
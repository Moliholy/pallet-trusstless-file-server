//! # Trustless File Server Pallet
//!
//! The Trustless File Server pallet provides functionality to store and serve files
//! along with the cryptographic proofs using [merkle trees](https://brilliant.org/wiki/merkle-tree/).
//!
//! The idea is to store file contents on the blockchain and serve them in chunks, along
//! with the corresponding cryptographic proof. With that proof, the client is able to compute back
//! the merkle root of the file, hence proving the files obtained in the chunk are correct.
//!
//! This methodology is pretty similar to how the Bittorrent protocol works. More information
//! about this protocol can be found [here](https://www.bittorrent.org/beps/bep_0030.html).
//!
//! This implementation has the following characteristics:
//! - It uses a fixed file chunk of 1KB
//! - The hashing algorithm is `sha256` instead of Bittorrent's original one: `sha1`.
//! - Files are stored on the blockchain. Bear in mind that this is an overall bad practice and
//! should be used with caution. A notable improvement to this pallet would actually be to
//! use IPFS or any other decentralized storage protocol to store files, and only keep on the
//! blockchain storage the associated hash.

#![cfg_attr(not(feature = "std"), no_std)]

extern crate core;

mod file_merkle_tree;

// Re-export pallet items so that they can be accessed from the crate namespace.
pub use pallet::*;

#[frame_support::pallet]
pub mod pallet {
    use frame_support::pallet_prelude::*;
    use frame_system::pallet_prelude::*;
    use sp_std::vec::Vec;
    use crate::file_merkle_tree::FileMerkleTree;

    #[pallet::pallet]
    #[pallet::without_storage_info]
    pub struct Pallet<T>(_);

    #[pallet::config]
    pub trait Config: frame_system::Config {
        /// Because this pallet emits events, it depends on the runtime's definition of an event.
        type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
    }


    #[pallet::event]
    #[pallet::generate_deposit(pub (super) fn deposit_event)]
    pub enum Event<T: Config> {
        /// Event emitted when a claim has been created.
        FileUploaded { who: T::AccountId, merkle_root: T::Hash, pieces: u32 },
    }

    #[pallet::error]
    pub enum Error<T> {
        /// Could not obtain the merkle root hash
        Unhasheable,
    }

    #[pallet::storage]
    pub(super) type Files<T: Config> = StorageMap<_, Blake2_128Concat, T::Hash, (T::AccountId, FileMerkleTree), OptionQuery>;

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        /// Uploads a file to the blockchain and computes its merkle tree.
        /// Bear in mind that as a general rule of thumb blockchains should not store big amounts of
        /// data, and instead decentralized services like IPFS should be used, storing only the
        /// associated hash on the blockchain.
        #[pallet::weight(0)]
        #[pallet::call_index(0)]
        pub fn upload_file(origin: OriginFor<T>, file_bytes: Vec<u8>) -> DispatchResult {
            // Check that the extrinsic was signed and get the signer.
            // This function will return an error if the extrinsic is not signed.
            let who = ensure_signed(origin)?;

            let file_merkle_tree = FileMerkleTree::new(file_bytes.clone());
            let merkle_root = T::Hash::decode(
                &mut file_merkle_tree.merkle_root()
            ).or(Err(Error::<T>::Unhasheable))?;

            // Store the claim with the sender and block number.
            Files::<T>::insert(&merkle_root, (&who, &file_merkle_tree));

            // Emit an event that the claim was created.
            Self::deposit_event(Event::FileUploaded {
                who,
                merkle_root,
                pieces: file_merkle_tree.pieces,
            });

            Ok(())
        }
    }

    // RPC methods
    impl<T: Config> Pallet<T> {
        /// Gets from the storage all file hashes ever submitted.
        pub fn get_files() -> Vec<(Vec<u8>, u32)> {
            let result = Files::<T>::iter()
                .map(|(_, (_, tree))| (tree.merkle_root().to_vec(), tree.pieces))
                .collect::<Vec<(Vec<u8>, u32)>>();
            result
        }

        /// Given a file's merkle root hash, gets the merkle proof of a given 1KB-chunk, identified
        /// by its position.
        /// Returns a tuple where the first element is the chunk content, and the second is
        /// the merkle proof.
        ///
        /// The idea is that the client can (and should) use the content to compute the sha256 hash,
        /// and with it hash along with the rest of the proofs until the merkle root is finally computed.
        /// This way it gets proven that the content is authentic in a trustless manner.
        pub fn get_proof(merkle_root: Vec<u8>, position: u32) -> Option<(Vec<u8>, Vec<Vec<u8>>)> {
            let key = T::Hash::decode(&mut merkle_root.as_slice()).map_err(|_| None::<T>).ok()?;
            let (_, merkle_tree) = Files::<T>::get(key)?;
            merkle_tree.merkle_proof(position)
        }
    }
}
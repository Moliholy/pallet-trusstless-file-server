use codec::Decode;
use frame_support::assert_ok;
use frame_system::ensure_signed;
use sp_io::hashing::sha2_256;
use sp_runtime::testing::H256;

use crate::mock::*;

#[test]
fn it_should_successfully_list_files_when_empty() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);
        let files = TrustlessFileServer::get_files();
        assert_eq!(files, []);
    });
}

#[test]
fn it_should_successfully_add_files() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);
        let bytes = include_bytes!("../img/substrate.png");
        let result = TrustlessFileServer::upload_file(RuntimeOrigin::signed(1), bytes.to_vec());
        assert_ok!(result);
        let files = TrustlessFileServer::get_files();
        assert_eq!(files.len(), 1);
        assert_eq!(
            files,
            [(
                [
                    24_u8, 211, 90, 77, 115, 30, 7, 133, 254, 133, 91, 78, 213, 144, 51, 233, 102,
                    7, 19, 125, 167, 187, 135, 92, 3, 246, 206, 165, 209, 248, 202, 207
                ]
                .to_vec(),
                12_u32
            )]
        );
    });
}

#[test]
fn it_should_successfully_get_proofs() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);
        let bytes = include_bytes!("../img/substrate.png");
        let result = TrustlessFileServer::upload_file(RuntimeOrigin::signed(1), bytes.to_vec());
        assert_ok!(result);
        let files = TrustlessFileServer::get_files();
        assert_eq!(files.len(), 1);
        let merkle_root = &TrustlessFileServer::get_files()[0].0;
        let proof = match TrustlessFileServer::get_proof(merkle_root.clone(), 0) {
            None => panic!("No proof found"),
            Some((_, siblings)) => siblings,
        };
        let key = H256::decode(&mut merkle_root.as_slice()).unwrap();
        let tree = TrustlessFileServer::get_file(key).unwrap().1;
        let chunk_size = tree.chunk_size();
        assert_eq!(chunk_size, 1024);
        let first_chunk = bytes.chunks(chunk_size).next().unwrap();
        let mut current = sha2_256(first_chunk).to_vec();
        assert_eq!(current, tree.file_chunk_hash_at(0).unwrap());
        for hash in proof {
            current = sha2_256(&[current, hash].concat()).to_vec();
        }
        assert_eq!(current.as_slice(), merkle_root);
    });
}

#[test]
fn should_have_the_correct_owner() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);
        let bytes = include_bytes!("../img/substrate.png");
        let owner = ensure_signed(RuntimeOrigin::signed(1)).unwrap();
        let result = TrustlessFileServer::upload_file(RuntimeOrigin::signed(1), bytes.to_vec());
        assert_ok!(result);

        let merkle_root = &TrustlessFileServer::get_files()[0].0;
        let key = H256::decode(&mut merkle_root.as_slice()).unwrap();
        assert_eq!(owner, TrustlessFileServer::get_file(key).unwrap().0);
    });
}

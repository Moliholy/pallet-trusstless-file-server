# Trustless File Server Pallet

This pallet intends to be a trustless file server similar to the [Bittorrent](https://www.bittorrent.org/beps/bep_0030.html) protocol.
The main idea is to make extensive use of [merkle trees](https://brilliant.org/wiki/merkle-tree/) to provide cryptographic proofs
of the file's validity.


## Limitations

- In the original Bittorrent protocol, the `sha1` hashing algorithm is used. However, in this implementation the `sha256` is used.
- Files are divided in chunks, with a **fixed chunk size of 1KB**.
- The whole files are stored on the blockchain storage. **This is a very severe limitation and an overall bad practice**. The original idea was
to store files on IPFS and only keep the corresponding hash on the blockchain. However, I found several limitations for using IPFS
in a substrate environment, so I finally decided to store the content directly on the blockchain. Further research would be needed
in order to fix this limitation.


## Walkthrough

This pallet implementation is composed of one extrinsic and two RPC methods. Tests have been performed using
[this file](./img/substrate.png).


### Extrinsics

#### uploadFile

This pallet call accepts the file bytes and uploads them to the blockchain (see [limitations](#limitations)), along with
its corresponding merkle tree and the number of file chunks.

![](./img/screenshot1.png "Uploading a file")

![](./img/screenshot2.png "Checking the file uploaded event")


### RPC methods

#### trustless_file_server_get_files

Returns a JSON list of the merkle hashes and number of 1KB pieces of the files being served. This operation simply
iterates through the `StorageMap` and fetches the corresponding data.

Request:
```shell
$ curl http://localhost:9933 -H "Content-Type:application/json;charset=utf-8" -d '{
      "jsonrpc": "2.0",
      "id": 1,
      "method": "trustless_file_server_get_files",
      "params": []
    }'
```

Response:
```json
{
  "jsonrpc": "2.0",
  "result": [
    {
      "merkle_root": "18d35a4d731e0785fe855b4ed59033e96607137da7bb875c03f6cea5d1f8cacf",
      "pieces": 12
    }
  ],
  "id": 1
}
```

#### trustless_file_server_get_proof

Returns the chunk's IPFS hash, along with the cryptographic proof necessary to build up the merkle root.

Request:
```shell
$ curl http://localhost:9933 -H "Content-Type:application/json;charset=utf-8" -d '{
      "jsonrpc": "2.0",
      "id": 1,
      "method": "trustless_file_server_get_proof",
      "params": [null, "18d35a4d731e0785fe855b4ed59033e96607137da7bb875c03f6cea5d1f8cacf", 8]
    }'
```

Response:
```json
{
  "jsonrpc": "2.0",
  "result": {
    "ipfs_hash": "bafkreihptszugz3ixlizu6eir5r4u5ygjzj55vews34bmur35jxgd3bwwm",
    "proof": [
      "72d2b6f941cb4954ece75eb4a4a10a5ee35e39575bf4e4397a3dd8b94c81a0a4",
      "f5a5fd42d16a20302798ef6ed309979b43003d2320d9f0e8ea9831a92759fb4b",
      "73b107c009c3044125c1f12015808b6adcfc44c473e013593f0ca1362bb80955",
      "fe98120ca95b4927928da36df60736b090a158d213c3fe2bb7683f27c90091ae"
    ]
  },
  "id": 1
}
```

##### Error handling:

This RPC method raises an error if the given piece does not exist or the merkle root is invalid.

Request:
```shell
$ curl http://localhost:9933 -H "Content-Type:application/json;charset=utf-8" -d '{
     "jsonrpc": "2.0",
      "id": 1,
      "method": "trustless_file_server_get_proof",
      "params": [null, "18d35a4d731e0785fe855b4ed59033e96607137da7bb875c03f6cea5d1f8cacf", 40]
    }'
```

```json
{
  "jsonrpc": "2.0",
  "error": {
    "code": 1,
    "message": "Runtime error",
    "data": "\"Failure getting the merkle proof\""
  },
  "id": 1
}
```
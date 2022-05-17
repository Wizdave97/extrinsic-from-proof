use codec::{Encode, Decode, Compact};
use frame_support::sp_runtime::traits::BlakeTwo256;
use sp_trie::{generate_trie_proof, TrieDBMut, TrieMut, StorageProof, Trie, verify_trie_proof};
use trie_db::proof::generate_proof;
use trie_db::TrieDB;

#[tokio::main]
async fn main() {
    let client = subxt::ClientBuilder::new()
        .set_url("ws://127.0.0.1:9988")
        .build::<subxt::DefaultConfig>()
        .await
        .unwrap();

    let block_hash = client.rpc().finalized_head().await.unwrap();
    let storage_prefix = frame_support::storage::storage_prefix(b"Timestamp", b"Now").to_vec();
    let storage_key = subxt::sp_core::storage::StorageKey(storage_prefix);
    let _correct_timestamp = client.rpc().storage(&storage_key, Some(block_hash)).await.unwrap().and_then(|val| u64::decode(&mut val.0.as_slice()).ok()).unwrap();

    let block = client.rpc().block(Some(block_hash)).await.unwrap().unwrap();
    let ext_root = block.block.header.extrinsics_root;

    let extrinsics = block
        .block
        .extrinsics
        .into_iter()
        .map(|e| e.encode())
        .collect::<Vec<_>>();
    let mut db = sp_trie::MemoryDB::<BlakeTwo256>::default();

    let root  = {
        let mut root = Default::default();
        let mut trie = <TrieDBMut<sp_trie::LayoutV0<BlakeTwo256>>>::new(&mut db, &mut root);

        for (i, ext) in extrinsics.clone().into_iter().enumerate() {
            let key = codec::Compact::<u32>(i as u32).encode();
            trie.insert(&key, &ext).unwrap();
        }
        *trie.root()
    };

    let key = codec::Compact(0u32).encode();
    println!("Key, {:?}", key);
    let trie = <TrieDB<sp_trie::LayoutV0<BlakeTwo256>>>::new(&db, &root).unwrap();

    let extrinsic_proof  = generate_proof::<_, sp_trie::LayoutV0<BlakeTwo256>, _, _ >(&trie, vec![&key]).unwrap();
    println!("Calculated Root == Header Ext Root: {}", root == ext_root);

    let timestamp_ext = extrinsics[0].clone();

    println!("Block hash {:?} \nTimestamp Extrinsic {:?}\n", block_hash, timestamp_ext);

    println!(
        "Proof {:?} \n",
        extrinsic_proof.iter().map(|n| hex::encode(n)).collect::<Vec<_>>()
    );

    // println!("\nEncode Call manually {:?}, timestamp : {:?}\n", (1u8, 0u8, codec::Compact(correct_timestamp)).encode(), correct_timestamp);

    println!("Extrinsic proof verification {:?}", verify_trie_proof::<sp_trie::LayoutV0<BlakeTwo256>, _, _, _>(&ext_root, &extrinsic_proof, vec![&(&key, Some(&*timestamp_ext))]));

    // Decode timestamp
   let timestamp = decode_timestamp_extrinsic(&extrinsic_proof, &ext_root);

    println!(
        "Block hash {:?} \n  Timestamp {:?}",
        block_hash,  timestamp
    ); 
}


/// Attempt to extract the timestamp extrinsic from the parachain header
pub fn decode_timestamp_extrinsic(proof: &[Vec<u8>], root: &sp_core::H256) -> u64 {
    let db = StorageProof::new(proof.to_vec()).into_memory_db::<BlakeTwo256>();
    let trie =
        sp_trie::TrieDB::<sp_trie::LayoutV0<BlakeTwo256>>::new(&db, root).unwrap();
    // Timestamp extrinsic should be the first inherent and hence the first extrinsic
    // https://github.com/paritytech/substrate/blob/d602397a0bbb24b5d627795b797259a44a5e29e9/primitives/trie/src/lib.rs#L99-L101
    let key = codec::Encode::encode(&Compact(0u32));
    let ext_bytes = trie
        .get(&key)
        .unwrap()
        .unwrap();

    println!("\nExtrinisic bytes {:?}", ext_bytes);
    // Decoding from the [2..] because the timestamp inherent has two extra bytes before the call that represents the
    // call length and the extrinsic version.
    let (_, _, timestamp): (u8, u8, Compact<u64>) =
        codec::Decode::decode(&mut &ext_bytes[2..]).unwrap();
    timestamp.into()
}
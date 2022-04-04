use codec::Encode;
use frame_support::sp_runtime::traits::BlakeTwo256;
use sp_trie::{generate_trie_proof, TrieDBMut, TrieMut};

#[tokio::main]
async fn main() {
    let client = subxt::ClientBuilder::new()
        .set_url("ws://127.0.0.1:9944")
        .build::<subxt::DefaultConfig>()
        .await
        .unwrap();

    let block_hash = client.rpc().finalized_head().await.unwrap();
    let storage_prefix = frame_support::storage::storage_prefix(b"Timestamp", b"Now").to_vec();
    let storage_key = subxt::sp_core::storage::StorageKey(storage_prefix);
    let storage_proof = client
        .rpc()
        .read_proof(vec![storage_key], Some(block_hash))
        .await
        .unwrap();
    let storage_proof = storage_proof
        .proof
        .into_iter()
        .map(|p| p.0)
        .collect::<Vec<_>>();

    let block = client.rpc().block(Some(block_hash)).await.unwrap().unwrap();

    let extrinsics = block
        .block
        .extrinsics
        .into_iter()
        .map(|e| e.encode())
        .collect::<Vec<_>>();
    let mut db = sp_trie::MemoryDB::<BlakeTwo256>::default();

    let root = {
        let mut root = Default::default();
        let mut trie = <TrieDBMut<sp_trie::LayoutV0<BlakeTwo256>>>::new(&mut db, &mut root);

        println!("Extrinsics {:?}", extrinsics);

        for (i, ext) in extrinsics.into_iter().enumerate() {
            let key = codec::Compact::<u32>(i as u32).encode();
            trie.insert(&key, &ext).unwrap();
        }
        trie.root().clone()
    };

    let key = codec::Compact::<u32>(0u32).encode();
    let extrinsic_proof =
        generate_trie_proof::<sp_trie::LayoutV0<BlakeTwo256>, _, _, _>(&db, root, vec![&key])
            .unwrap();

    println!(
        "Approximate Storage proof size is {:?} bytes",
        proof_size(storage_proof)
    );
    println!(
        "Approximate Extrinsic proof size is {:?} bytes",
        proof_size(extrinsic_proof)
    )
}

fn proof_size(proof: Vec<Vec<u8>>) -> usize {
    proof.into_iter().fold(0, |acc, p| acc + p.len())
}

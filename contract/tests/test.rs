use near_crypto::{KeyType, SecretKey, Signature};

#[tokio::test]
async fn test_create_wallet() -> Result<(), Box<dyn std::error::Error>> {
    let contract_wasm = near_workspaces::compile_project("./").await?;
    let sandbox = near_workspaces::sandbox().await?;
    let contract = sandbox.dev_deploy(&contract_wasm).await?;

    let recovery_key = SecretKey::from_random(KeyType::ED25519);
    let account_key = SecretKey::from_random(KeyType::ED25519);
    let signature = recovery_key.sign(format!("I want to log in to [REDACTED] Explorer and add key {} for quick trading without confirmation in wallet", account_key.public_key().to_string()).as_bytes());
    let outcome = contract
        .call("new")
        .args_json(serde_json::json!({
            "recovery_key": recovery_key.public_key().key_data(),
            "public_key": account_key.public_key().key_data(),
            "signature": if let Signature::ED25519(signature) = signature { signature.to_bytes().to_vec() } else { unreachable!() },
        }))
        .transact()
        .await?;
    println!("{outcome:?}");
    assert!(outcome.is_success());
    Ok(())
}

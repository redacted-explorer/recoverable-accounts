use std::net::SocketAddrV4;

use near_api::signer::secret_key::SecretKeySigner;
use near_api::signer::Signer;
use near_api::{Contract, Transaction};
use near_crypto::{PublicKey, SecretKey};
use near_gas::NearGas;
use near_primitives::account::AccessKey;
use near_primitives::action::{
    Action, AddKeyAction, CreateAccountAction, DeployContractAction, TransferAction,
};
use near_primitives::types::AccountId;
use near_primitives::views::ExecutionStatusView;
use near_token::NearToken;
use warp::reply::Reply;
use warp::Filter;

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();
    simple_logger::init().unwrap();

    let cors = warp::cors()
        .allow_any_origin()
        .allow_headers(vec![
            "content-type",
        ])
        .allow_methods(vec!["GET", "POST", "OPTIONS"]);

    let create = warp::path("create")
        .and(warp::post())
        .and(warp::body::json::<CreateRequest>())
        .and_then(create_account)
        .with(cors.clone());
    let recover = warp::path("recover")
        .and(warp::post())
        .and(warp::body::json::<RecoverRequest>())
        .and_then(recover_account)
        .with(cors.clone());

    let routes = create.or(recover);

    if let Ok(tls_options) = std::env::var("TLS") {
        let cert_path = tls_options.split(':').collect::<Vec<&str>>()[0];
        let key_path = tls_options.split(':').collect::<Vec<&str>>()[1];
        warp::serve(routes)
            .tls()
            .cert_path(cert_path)
            .key_path(key_path)
            .run(([0, 0, 0, 0], 443))
            .await;
    } else if let Ok(addrs) = std::env::var("BIND") {
        warp::serve(routes)
            .run(addrs.parse::<SocketAddrV4>().unwrap())
            .await;
    } else {
        warp::serve(routes).run(([127, 0, 0, 1], 3030)).await;
    }
}

#[derive(Debug, serde::Deserialize)]
struct CreateRequest {
    pub recovery_chain_type: ChainType,
    pub name: String,
    pub recovery_key: Vec<u8>,
    pub public_key: PublicKey,
    pub signature: Vec<u8>,
}

#[derive(Debug, serde::Deserialize)]
enum ChainType {
    Near,
    Evm,
    Solana,
}

async fn create_account(query: CreateRequest) -> Result<impl Reply, warp::Rejection> {
    log::info!("Received create request: {query:?}");

    // TODO check signature

    let operator_private_key: SecretKey = std::env::var("OPERATOR_PRIVATE_KEY")
        .unwrap()
        .parse()
        .unwrap();
    let operator_account_id: AccountId = match query.recovery_chain_type {
        ChainType::Near => std::env::var("PARENT_ACCOUNT_ID_NEAR")
            .unwrap()
            .parse()
            .unwrap(),
        ChainType::Evm => std::env::var("PARENT_ACCOUNT_ID_EVM")
            .unwrap()
            .parse()
            .unwrap(),
        ChainType::Solana => std::env::var("PARENT_ACCOUNT_ID_SOLANA")
            .unwrap()
            .parse()
            .unwrap(),
    };
    let Ok(user_account_id) = format!("{}.{operator_account_id}", query.name).parse::<AccountId>()
    else {
        return Ok(warp::reply::json(&serde_json::json!({
            "error": "invalid account name"
        })));
    };

    // TODO check if account exists

    let tx = Transaction::construct(operator_account_id.clone(), user_account_id.clone())
        .add_actions(vec![
            Action::CreateAccount(CreateAccountAction {}),
            Action::Transfer(TransferAction {
                deposit: 1_229_085_000_000_000_000_000_000,
            }),
            Action::AddKey(Box::new(AddKeyAction {
                public_key: operator_private_key.public_key(),
                access_key: AccessKey::full_access(),
            })),
            Action::DeployContract(DeployContractAction {
                code: include_bytes!("../../contract/target/near/recoverable_account.wasm")
                    .to_vec(),
            }),
            Action::FunctionCall(Box::new(near_primitives::transaction::FunctionCallAction {
                method_name: "new".to_string(),
                args: serde_json::to_vec(&serde_json::json!({
                    "recovery_key": query.recovery_key,
                    "public_key": query.public_key,
                    "signature": query.signature,
                }))
                .unwrap(),
                gas: 10_000_000_000_000,
                deposit: 0,
            })),
        ])
        .with_signer(Signer::new(SecretKeySigner::new(operator_private_key)).unwrap())
        .send_to_mainnet()
        .await;

    log::info!(
        "Create tx: {}",
        match &tx {
            Ok(tx) => format!("Ok, {}", tx.transaction.hash),
            Err(err) => format!("{err:?}"),
        }
    );

    if let Err(err) = &tx {
        return Ok(warp::reply::json(&serde_json::json!({
            "error": format!("{err}")
        })));
    }
    let tx = tx.unwrap();

    if let ExecutionStatusView::Failure(err) = &tx.transaction_outcome.outcome.status {
        return Ok(warp::reply::json(&serde_json::json!({
            "error": format!("Execution error in transaction {}: {err:?}", tx.transaction.hash)
        })));
    }

    Ok(warp::reply::json(&serde_json::json!({
        "account_id": user_account_id,
    })))
}

#[derive(Debug, serde::Deserialize)]
struct RecoverRequest {
    pub account_id: AccountId,
    pub new_public_key: PublicKey,
    pub signature: Vec<u8>,
}

async fn recover_account(query: RecoverRequest) -> Result<impl Reply, warp::Rejection> {
    log::info!("Received recover request: {query:?}");

    // TODO check signature

    let operator_private_key: SecretKey = std::env::var("OPERATOR_PRIVATE_KEY")
        .unwrap()
        .parse()
        .unwrap();
    let operator_account_id: AccountId = std::env::var("OPERATOR_ACCOUNT_ID")
        .unwrap()
        .parse()
        .unwrap();

    let tx = Contract(query.account_id)
        .call_function(
            "recover",
            serde_json::json!({
                "public_key": query.new_public_key,
                "recovery_signature": query.signature,
            }),
        )
        .unwrap()
        .transaction()
        .deposit(NearToken::from_yoctonear(0))
        .gas(NearGas::from_tgas(100))
        .with_signer(
            operator_account_id,
            Signer::new(SecretKeySigner::new(operator_private_key)).unwrap(),
        )
        .send_to_mainnet()
        .await;

    log::info!(
        "Recover tx: {}",
        match &tx {
            Ok(tx) => format!("Ok, {}", tx.transaction.hash),
            Err(err) => format!("{err:?}"),
        }
    );

    if let Err(err) = &tx {
        return Ok(warp::reply::json(&serde_json::json!({
            "error": format!("{err}")
        })));
    }
    let tx = tx.unwrap();

    if let ExecutionStatusView::Failure(err) = &tx.transaction_outcome.outcome.status {
        return Ok(warp::reply::json(&serde_json::json!({
            "error": format!("Execution error in transaction {}: {err:?}", tx.transaction.hash)
        })));
    }

    Ok(warp::reply::json(&serde_json::json!({
        "ok": true,
    })))
}

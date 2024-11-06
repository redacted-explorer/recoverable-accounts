use near_sdk::{
    bs58, env, near, require, AccountId, Allowance, CurveType, Gas, GasWeight, NearToken,
    PanicOnDefault, Promise, PublicKey,
};

#[near(contract_state)]
#[derive(PanicOnDefault)]
pub struct Contract {
    recovery_key: [u8; 32],
    user_key: [u8; 32],
}

#[near]
impl Contract {
    #[init]
    pub fn new(recovery_key: [u8; 32], public_key: [u8; 32], signature: Vec<u8>) -> Self {
        let Ok(signature): Result<[u8; 64], _> = signature.try_into() else {
            env::panic_str("Invalid signature")
        };
        require!(
            verify_signature(&recovery_key, format!("I want to log in to [REDACTED] Explorer and add key ed25519:{} for quick trading without confirmation in wallet", bs58::encode(public_key).into_string()).as_bytes(), &signature),
            "Invalid signature"
        );
        Promise::new(env::current_account_id()).add_access_key_allowance(
            if let Ok(key) = PublicKey::from_parts(CurveType::ED25519, public_key.to_vec()) {
                key
            } else {
                env::panic_str("Invalid public key, only ED25519 is supported")
            },
            Allowance::Unlimited,
            env::current_account_id(),
            "relay_transactions".to_string(),
        );
        Self {
            recovery_key,
            user_key: public_key,
        }
    }

    pub fn get_recovery_key(&self) -> Vec<u8> {
        self.recovery_key.to_vec()
    }

    #[private]
    pub fn relay_transactions(&mut self, transactions: Vec<RelayedTransaction>) -> Promise {
        require!(!transactions.is_empty(), "No transactions to relay");
        let mut promise = Promise::new(transactions[0].receiver_id.clone()).function_call_weight(
            transactions[0].method.clone(),
            transactions[0].args.clone(),
            transactions[0].deposit,
            Gas::from_gas(1),
            GasWeight::default(),
        );
        for transaction in transactions.into_iter().skip(1) {
            promise = promise.then(Promise::new(transaction.receiver_id).function_call_weight(
                transaction.method,
                transaction.args,
                transaction.deposit,
                Gas::from_gas(1),
                GasWeight::default(),
            ));
        }
        promise
    }

    pub fn recover(&mut self, new_public_key: [u8; 32], recovery_signature: Vec<u8>) -> Promise {
        let Ok(recovery_signature): Result<[u8; 64], _> = recovery_signature.try_into() else {
            env::panic_str("Invalid signature")
        };
        require!(
            verify_signature(
                &self.recovery_key,
                format!("I want to log in to [REDACTED] Explorer and add key ed25519:{} for quick trading without confirmation in wallet", bs58::encode(new_public_key).into_string()).as_bytes(),
                &recovery_signature
            ),
            "Invalid signature"
        );
        self.user_key = new_public_key;
        Promise::new(env::current_account_id())
            .delete_key(
                if let Ok(key) = PublicKey::from_parts(CurveType::ED25519, self.user_key.to_vec()) {
                    key
                } else {
                    env::panic_str("Invalid public key, only ED25519 is supported")
                },
            )
            .then(
                Promise::new(env::current_account_id()).add_access_key_allowance(
                    if let Ok(key) =
                        PublicKey::from_parts(CurveType::ED25519, new_public_key.to_vec())
                    {
                        key
                    } else {
                        env::panic_str("Invalid public key, only ED25519 is supported")
                    },
                    Allowance::Unlimited,
                    env::current_account_id(),
                    "relay_transactions".to_string(),
                ),
            )
    }
}

fn verify_signature(
    public_key: &[u8; 32],
    message: impl Into<Vec<u8>>,
    signature: &[u8; 64],
) -> bool {
    let message = message.into();
    env::ed25519_verify(signature, &message, public_key)
}

#[near(serializers=[json])]
pub struct RelayedTransaction {
    pub receiver_id: AccountId,
    pub method: String,
    pub args: Vec<u8>,
    pub deposit: NearToken,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_verify_signature_positive() {
        let public_key = [
            75, 188, 143, 161, 187, 53, 78, 9, 209, 243, 173, 201, 92, 239, 81, 65, 68, 163, 106,
            64, 5, 95, 41, 254, 248, 55, 110, 52, 236, 34, 158, 165,
        ];
        let message = "I want to log in to [REDACTED] Explorer and add key ed25519:Ae3nRD5NNd7EGJqs3PmCCX5EsAycUGJxk3x9LoHpVtXu for quick trading without confirmation in wallet".as_bytes();
        let signature = [
            145, 195, 102, 41, 132, 32, 30, 85, 11, 218, 120, 160, 178, 180, 8, 157, 107, 132, 238,
            152, 63, 63, 115, 181, 111, 87, 140, 244, 200, 193, 2, 58, 92, 102, 58, 197, 35, 230,
            0, 61, 165, 174, 210, 165, 103, 233, 173, 81, 81, 86, 93, 26, 161, 225, 211, 143, 95,
            3, 171, 173, 81, 18, 39, 7,
        ];
        assert!(verify_signature(&public_key, message, &signature));
    }

    #[test]
    fn test_verify_signature_negative() {
        let public_key = [
            75, 188, 143, 161, 187, 53, 78, 9, 209, 243, 173, 201, 92, 239, 81, 65, 68, 163, 106,
            64, 5, 95, 41, 254, 248, 55, 110, 52, 236, 34, 158, 165,
        ];
        let message = "I want to log in to [REDACTED] Explorer and add key ed25519:Ae3nRD5NNd7EGJqs3PmCCX5EsAycUGJxk3x9LoHpVtXu for quick trading without confirmation in wallet".as_bytes();
        let signature = [
            145, 195, 102, 41, 132, 32, 30, 85, 11, 218, 120, 160, 178, 180, 8, 157, 107, 132, 238,
            152, 63, 63, 115, 181, 111, 87, 140, 244, 200, 193, 2, 58, 92, 102, 58, 197, 35, 230,
            0, 61, 165, 174, 210, 165, 103, 233, 173, 81, 81, 86, 93, 26, 161, 225, 211, 143, 95,
            3, 171, 173, 81, 18, 39, 8, // changed 7 to 8
        ];
        assert!(!verify_signature(&public_key, message, &signature));
    }
}

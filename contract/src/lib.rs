use near_sdk::{
    env, near, require, AccountId, Allowance, Gas, GasWeight, NearToken, PanicOnDefault, Promise,
    PublicKey,
};

#[near(contract_state)]
#[derive(PanicOnDefault)]
pub struct Contract {
    recovery_key: Vec<u8>,
    user_key: Vec<u8>,
}

#[near]
impl Contract {
    #[init]
    pub fn new(recovery_key: Vec<u8>, public_key: PublicKey, signature: Vec<u8>) -> Self {
        // TODO: check signature
        Promise::new(env::current_account_id()).add_access_key_allowance(
            public_key.clone(),
            Allowance::Unlimited,
            env::current_account_id(),
            "relay_transactions".to_string(),
        );
        Self {
            recovery_key,
            user_key: public_key.into_bytes(),
        }
    }

    pub fn get_recovery_key(&self) -> Vec<u8> {
        self.recovery_key.clone()
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

    pub fn recover(&mut self, public_key: PublicKey, recovery_signature: Vec<u8>) -> Promise {
        // TODO: check signature
        let old_key = self.user_key.clone().try_into().unwrap();
        self.user_key = public_key.clone().into_bytes();
        Promise::new(env::current_account_id())
            .delete_key(old_key)
            .then(
                Promise::new(env::current_account_id()).add_access_key_allowance(
                    public_key,
                    Allowance::Unlimited,
                    env::current_account_id(),
                    "relay_transactions".to_string(),
                ),
            )
    }
}

#[near(serializers=[json])]
pub struct RelayedTransaction {
    pub receiver_id: AccountId,
    pub method: String,
    pub args: Vec<u8>,
    pub deposit: NearToken,
}

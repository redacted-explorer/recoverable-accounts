use near_sdk::{
    env, near, AccountId, Allowance, Gas, GasWeight, NearToken, PanicOnDefault, Promise, PublicKey,
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
            "relay_transaction".to_string(),
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
    pub fn relay_transaction(
        &mut self,
        receiver_id: AccountId,
        method: String,
        args: Vec<u8>,
        deposit: NearToken,
    ) -> Promise {
        Promise::new(receiver_id).function_call_weight(
            method,
            args,
            deposit,
            Gas::from_gas(1),
            GasWeight::default(),
        )
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
                    "relay_transaction".to_string(),
                ),
            )
    }
}

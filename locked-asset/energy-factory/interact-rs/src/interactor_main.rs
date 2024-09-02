#![allow(non_snake_case)]
#![allow(dead_code)]
#![allow(unused)]

mod proxy;

use multiversx_sc_snippets::imports::*;
use multiversx_sc_snippets::sdk;
use proxy::Energy;
use serde::{Deserialize, Serialize};
use std::{
    io::{Read, Write},
    path::Path,
};

const GATEWAY: &str = sdk::blockchain::DEVNET_GATEWAY;
const STATE_FILE: &str = "state.toml";
const OLD_LOCKED_ASSET_FACTORY_ADDRESS: &str =
    "erd1qqqqqqqqqqqqqpgqxk5hlvgkwzen6q0kxgaljtu6p524swwcv5ysa9hnht";
const BASE_ASSET_TOKEN_ID: &str = "TST-af9b21";
const LEGACY_TOKEN_ID: &str = "TSTT-d96162";
const ANOTHER_TOKEN_ID: &str = "SMTH-6fb124";
const ALICE_ADDRESS: &str = "erd1qyu5wthldzr8wx5c9ucg8kjagg0jfs53s8nr3zpz3hypefsdd8ssycr6th";
const BOB_ADDRESS: &str = "erd1spyavw0956vq68xj8y4tenjpq2wd5a9p2c6j8gsz7ztyrnpxrruqzu66jx";

#[tokio::main]
async fn main() {
    env_logger::init();

    let mut args = std::env::args();
    let _ = args.next();
    let cmd = args.next().expect("at least one argument required");
    let mut interact = ContractInteract::new().await;
    match cmd.as_str() {
        // "deploy" => interact.deploy().await,
        // "lockTokens" => interact.lock_tokens_endpoint().await,
        // "unlockTokens" => interact.unlock_tokens_endpoint().await,
        // "extendLockPeriod" => interact.extend_lock_period().await,
        // "issueLockedToken" => interact.issue_locked_token().await,
        // "getLockedTokenId" => interact.locked_token().await,
        // "getBaseAssetTokenId" => interact.base_asset_token_id().await,
        // "getLegacyLockedTokenId" => interact.legacy_locked_token_id().await,
        // "getEnergyEntryForUser" => interact.get_updated_energy_entry_for_user().await,
        // "getEnergyAmountForUser" => interact.get_energy_amount_for_user().await,
        // "addLockOptions" => interact.add_lock_options().await,
        // "getLockOptions" => interact.get_lock_options_view().await,
        // "unlockEarly" => interact.unlock_early().await,
        // "reduceLockPeriod" => interact.reduce_lock_period().await,
        // "getPenaltyAmount" => interact.calculate_penalty_amount().await,
        // "setTokenUnstakeAddress" => interact.set_token_unstake_address().await,
        // "revertUnstake" => interact.revert_unstake().await,
        // "getTokenUnstakeScAddress" => interact.token_unstake_sc_address().await,
        // "setEnergyForOldTokens" => interact.set_energy_for_old_tokens().await,
        // "updateEnergyAfterOldTokenUnlock" => interact.update_energy_after_old_token_unlock().await,
        // "migrateOldTokens" => interact.migrate_old_tokens().await,
        // "pause" => interact.pause_endpoint().await,
        // "unpause" => interact.unpause_endpoint().await,
        // "isPaused" => interact.paused_status().await,
        // "setTransferRoleLockedToken" => interact.set_transfer_role().await,
        // "setBurnRoleLockedToken" => interact.set_burn_role().await,
        // "mergeTokens" => interact.merge_tokens_endpoint().await,
        // "lockVirtual" => interact.lock_virtual().await,
        // "addSCAddressToWhitelist" => interact.add_sc_address_to_whitelist().await,
        // "removeSCAddressFromWhitelist" => interact.remove_sc_address_from_whitelist().await,
        // "isSCAddressWhitelisted" => interact.is_sc_address_whitelisted().await,
        // "addToTokenTransferWhitelist" => interact.add_to_token_transfer_whitelist().await,
        // "removeFromTokenTransferWhitelist" => interact.remove_from_token_transfer_whitelist().await,
        // "setUserEnergyAfterLockedTokenTransfer" => {
        //     interact.set_user_energy_after_locked_token_transfer().await
        // }
        _ => panic!("unknown command: {}", &cmd),
    }
}

#[derive(Debug, Default, Serialize, Deserialize)]
struct State {
    contract_address: Option<Bech32Address>,
}

impl State {
    // Deserializes state from file
    pub fn load_state() -> Self {
        if Path::new(STATE_FILE).exists() {
            let mut file = std::fs::File::open(STATE_FILE).unwrap();
            let mut content = String::new();
            file.read_to_string(&mut content).unwrap();
            toml::from_str(&content).unwrap()
        } else {
            Self::default()
        }
    }

    /// Sets the contract address
    pub fn set_address(&mut self, address: Bech32Address) {
        self.contract_address = Some(address);
    }

    /// Returns the contract address
    pub fn current_address(&self) -> &Bech32Address {
        self.contract_address
            .as_ref()
            .expect("no known contract, deploy first")
    }
}

impl Drop for State {
    // Serializes state to file
    fn drop(&mut self) {
        let mut file = std::fs::File::create(STATE_FILE).unwrap();
        file.write_all(toml::to_string(self).unwrap().as_bytes())
            .unwrap();
    }
}

struct ContractInteract {
    interactor: Interactor,
    wallet_address: Address,
    second_user: Address,
    contract_code: BytesValue,
    state: State,
}

impl ContractInteract {
    async fn new() -> Self {
        let mut interactor = Interactor::new(GATEWAY).await;
        let wallet_address = interactor.register_wallet(test_wallets::alice());
        let second_user = interactor.register_wallet(test_wallets::bob());

        let contract_code = BytesValue::interpret_from(
            "mxsc:../output/energy-factory.mxsc.json",
            &InterpreterContext::default(),
        );

        ContractInteract {
            interactor,
            wallet_address,
            second_user,
            contract_code,
            state: State::load_state(),
        }
    }

    async fn deploy(
        &mut self,
        base_asset_token_id: &str,
        legacy_token_id: &str,
        old_locked_asset_factory_address: &str,
        min_migrated_token_locked_period: u64,
        lock_options: Vec<(u64, u64)>,
    ) {
        let base_asset_token_id = TokenIdentifier::from_esdt_bytes(base_asset_token_id.as_bytes());
        let legacy_token_id = TokenIdentifier::from_esdt_bytes(legacy_token_id.as_bytes());
        let old_locked_asset_factory_address = bech32::decode(old_locked_asset_factory_address);
        let lock_options = MultiValueVec::from(
            lock_options
                .iter()
                .map(|(a, b)| MultiValue2::from((*a, *b)))
                .collect::<Vec<_>>(),
        );

        let new_address = self
            .interactor
            .tx()
            .from(&self.wallet_address)
            .gas(200_000_000u64)
            .typed(proxy::SimpleLockEnergyProxy)
            .init(
                base_asset_token_id,
                legacy_token_id,
                old_locked_asset_factory_address,
                min_migrated_token_locked_period,
                lock_options,
            )
            .code(&self.contract_code)
            .returns(ReturnsNewAddress)
            .prepare_async()
            .run()
            .await;
        let new_address_bech32 = bech32::encode(&new_address);
        self.state.set_address(Bech32Address::from_bech32_string(
            new_address_bech32.clone(),
        ));

        println!("new address: {new_address_bech32}");
    }

    async fn lock_tokens_endpoint(
        &mut self,
        lock_epochs: u64,
        opt_destination: &str,
        token_id: &str,
        token_nonce: u64,
        token_amount: u128,
    ) {
        let token_id = String::from(token_id);
        let token_amount = BigUint::<StaticApi>::from(token_amount);
        let opt_destination = OptionalValue::Some(bech32::decode(opt_destination));

        let response = self
            .interactor
            .tx()
            .from(&self.wallet_address)
            .gas(120_000_000u64)
            .to(self.state.current_address())
            .typed(proxy::SimpleLockEnergyProxy)
            .lock_tokens_endpoint(lock_epochs, opt_destination)
            .payment((
                TokenIdentifier::from(token_id.as_str()),
                token_nonce,
                token_amount,
            ))
            .returns(ReturnsResultUnmanaged)
            .prepare_async()
            .run()
            .await;

        println!("Result: {response:?}");
    }

    async fn lock_tokens_endpoint_fail(
        &mut self,
        lock_epochs: u64,
        opt_destination: &str,
        token_id: &str,
        token_nonce: u64,
        token_amount: u128,
        expected_result: ExpectError<'_>,
    ) {
        let token_id = String::from(token_id);
        let token_amount = BigUint::<StaticApi>::from(token_amount);
        let opt_destination = OptionalValue::Some(bech32::decode(opt_destination));

        let response = self
            .interactor
            .tx()
            .from(&self.wallet_address)
            .gas(120_000_000u64)
            .to(self.state.current_address())
            .typed(proxy::SimpleLockEnergyProxy)
            .lock_tokens_endpoint(lock_epochs, opt_destination)
            .payment((
                TokenIdentifier::from(token_id.as_str()),
                token_nonce,
                token_amount,
            ))
            .returns(expected_result)
            .prepare_async()
            .run()
            .await;

        println!("Result: {response:?}");
    }

    async fn unlock_tokens_endpoint(
        &mut self,
        token_id: &str,
        token_nonce: u64,
        token_amount: u128,
    ) {
        let token_id = String::from(token_id);
        let token_amount = BigUint::<StaticApi>::from(token_amount);

        let response = self
            .interactor
            .tx()
            .from(&self.wallet_address)
            .to(self.state.current_address())
            .typed(proxy::SimpleLockEnergyProxy)
            .unlock_tokens_endpoint()
            .payment((
                TokenIdentifier::from(token_id.as_str()),
                token_nonce,
                token_amount,
            ))
            .returns(ReturnsResultUnmanaged)
            .prepare_async()
            .run()
            .await;

        println!("Result: {response:?}");
    }

    async fn unlock_tokens_endpoint_fail(
        &mut self,
        token_id: &str,
        token_nonce: u64,
        token_amount: u128,
        expected_result: ExpectError<'_>,
    ) {
        let token_id = String::from(token_id);
        let token_amount = BigUint::<StaticApi>::from(token_amount);

        let response = self
            .interactor
            .tx()
            .from(&self.wallet_address)
            .to(self.state.current_address())
            .typed(proxy::SimpleLockEnergyProxy)
            .unlock_tokens_endpoint()
            .payment((
                TokenIdentifier::from(token_id.as_str()),
                token_nonce,
                token_amount,
            ))
            .returns(expected_result)
            .prepare_async()
            .run()
            .await;

        println!("Result: {response:?}");
    }

    async fn unlock_tokens_endpoint_no_payments(&mut self) {
        let response = self
            .interactor
            .tx()
            .from(&self.wallet_address)
            .to(self.state.current_address())
            .typed(proxy::SimpleLockEnergyProxy)
            .unlock_tokens_endpoint()
            .returns(ExpectError(4, "No payments"))
            .prepare_async()
            .run()
            .await;
    }

    async fn unlock_tokens_endpoint_diff_payments(&mut self) {
        let response = self
            .interactor
            .tx()
            .from(&self.wallet_address)
            .gas(120_000_000u64)
            .to(self.state.current_address())
            .typed(proxy::SimpleLockEnergyProxy)
            .unlock_tokens_endpoint()
            .egld_or_multi_esdt(EgldOrMultiEsdtPayment::MultiEsdt(ManagedVec::from(vec![
                EsdtTokenPayment::new(
                    TokenIdentifier::from(BASE_ASSET_TOKEN_ID),
                    0u64,
                    BigUint::<StaticApi>::from(1u128),
                ),
                EsdtTokenPayment::new(
                    TokenIdentifier::from(LEGACY_TOKEN_ID),
                    0u64,
                    BigUint::<StaticApi>::from(1u128),
                ),
            ])))
            // .payment((
            //     TokenIdentifier::from(BASE_ASSET_TOKEN_ID),
            //     0u64,
            //     BigUint::<StaticApi>::from(1u128),
            // ))
            .returns(ExpectError(4, "Invalid payment token"))
            .prepare_async()
            .run()
            .await;
    }

    async fn extend_lock_period(
        &mut self,
        lock_epochs: u64,
        user: &str,
        token_id: &str,
        token_nonce: u64,
        token_amount: u128,
    ) {
        let token_id = String::from(token_id);
        let token_amount = BigUint::<StaticApi>::from(token_amount);
        let user = bech32::decode(user);

        let response = self
            .interactor
            .tx()
            .from(&self.wallet_address)
            .gas(120_000_000u64)
            .to(self.state.current_address())
            .typed(proxy::SimpleLockEnergyProxy)
            .extend_lock_period(lock_epochs, user)
            .payment((
                TokenIdentifier::from(token_id.as_str()),
                token_nonce,
                token_amount,
            ))
            .returns(ReturnsResultUnmanaged)
            .prepare_async()
            .run()
            .await;

        println!("Result: {response:?}");
    }

    async fn extend_lock_period_fail(
        &mut self,
        caller: &Bech32Address,
        lock_epochs: u64,
        user: &str,
        token_id: &str,
        token_nonce: u64,
        token_amount: u128,
        expected_result: ExpectError<'_>,
    ) {
        let token_id = String::from(token_id);
        let token_amount = BigUint::<StaticApi>::from(token_amount);
        let user = bech32::decode(user);

        let response = self
            .interactor
            .tx()
            .from(caller)
            .gas(120_000_000u64)
            .to(self.state.current_address())
            .typed(proxy::SimpleLockEnergyProxy)
            .extend_lock_period(lock_epochs, user)
            .payment((
                TokenIdentifier::from(token_id.as_str()),
                token_nonce,
                token_amount,
            ))
            .returns(expected_result)
            .prepare_async()
            .run()
            .await;

        println!("Result: {response:?}");
    }

    async fn issue_locked_token(
        &mut self,
        egld_amount: u128,
        token_display_name: &str,
        token_ticker: &str,
        num_decimals: u32,
    ) {
        let egld_amount = BigUint::<StaticApi>::from(egld_amount);

        let token_display_name = ManagedBuffer::new_from_bytes(token_display_name.as_bytes());
        let token_ticker = ManagedBuffer::new_from_bytes(token_ticker.as_bytes());

        let response = self
            .interactor
            .tx()
            .from(&self.wallet_address)
            .gas(120_000_000u64)
            .to(self.state.current_address())
            .typed(proxy::SimpleLockEnergyProxy)
            .issue_locked_token(token_display_name, token_ticker, num_decimals)
            .egld(egld_amount)
            .returns(ReturnsResultUnmanaged)
            .prepare_async()
            .run()
            .await;

        println!("Result: {response:?}");
    }

    async fn locked_token(&mut self) {
        let result_value = self
            .interactor
            .query()
            .to(self.state.current_address())
            .typed(proxy::SimpleLockEnergyProxy)
            .locked_token()
            .returns(ReturnsResultUnmanaged)
            .prepare_async()
            .run()
            .await;

        println!("Result: {result_value:?}");
    }

    async fn base_asset_token_id(&mut self) {
        let result_value = self
            .interactor
            .query()
            .to(self.state.current_address())
            .typed(proxy::SimpleLockEnergyProxy)
            .base_asset_token_id()
            .returns(ReturnsResultUnmanaged)
            .prepare_async()
            .run()
            .await;

        println!("Result: {result_value:?}");
    }

    async fn legacy_locked_token_id(&mut self) {
        let result_value = self
            .interactor
            .query()
            .to(self.state.current_address())
            .typed(proxy::SimpleLockEnergyProxy)
            .legacy_locked_token_id()
            .returns(ReturnsResultUnmanaged)
            .prepare_async()
            .run()
            .await;

        println!("Result: {result_value:?}");
    }

    async fn get_updated_energy_entry_for_user(&mut self) -> (i64, u64, u64) {
        let user = bech32::decode("");

        let result_value = self
            .interactor
            .query()
            .to(self.state.current_address())
            .typed(proxy::SimpleLockEnergyProxy)
            .get_updated_energy_entry_for_user(user)
            .returns(ReturnsResultUnmanaged)
            .prepare_async()
            .run()
            .await;

        return (
            result_value.amount.to_i64().unwrap(),
            result_value.last_update_epoch,
            result_value.total_locked_tokens.to_u64().unwrap(),
        );
    }

    async fn get_energy_amount_for_user(&mut self) {
        let user = bech32::decode("");

        let result_value = self
            .interactor
            .query()
            .to(self.state.current_address())
            .typed(proxy::SimpleLockEnergyProxy)
            .get_energy_amount_for_user(user)
            .returns(ReturnsResultUnmanaged)
            .prepare_async()
            .run()
            .await;

        println!("Result: {result_value:?}");
    }

    async fn add_lock_options(&mut self, lock_options: Vec<(u64, u64)>) {
        let new_lock_options = MultiValueVec::from(
            lock_options
                .iter()
                .map(|(a, b)| MultiValue2::from((*a, *b)))
                .collect::<Vec<_>>(),
        );
        let response = self
            .interactor
            .tx()
            .from(&self.wallet_address)
            .to(self.state.current_address())
            .typed(proxy::SimpleLockEnergyProxy)
            .add_lock_options(new_lock_options)
            .returns(ReturnsResultUnmanaged)
            .prepare_async()
            .run()
            .await;

        println!("Result: {response:?}");
    }
    async fn add_lock_options_fail(
        &mut self,
        caller: &Bech32Address,
        lock_options: Vec<(u64, u64)>,
        expected_result: ExpectError<'_>,
    ) {
        let new_lock_options = MultiValueVec::from(
            lock_options
                .iter()
                .map(|(a, b)| MultiValue2::from((*a, *b)))
                .collect::<Vec<_>>(),
        );
        let response = self
            .interactor
            .tx()
            .from(caller)
            .to(self.state.current_address())
            .typed(proxy::SimpleLockEnergyProxy)
            .add_lock_options(new_lock_options)
            .returns(expected_result)
            .prepare_async()
            .run()
            .await;

        println!("Result: {response:?}");
    }

    // async fn get_lock_options_view(&mut self) {
    //     let result_value = self
    //         .interactor
    //         .query()
    //         .to(self.state.current_address())
    //         .typed(proxy::SimpleLockEnergyProxy)
    //         .get_lock_options_view()
    //         .returns(ReturnsResultUnmanaged)
    //         .prepare_async()
    //         .run()
    //         .await;

    //     println!("Result: {result_value:?}");
    // }

    async fn unlock_early(&mut self) {
        let token_id = String::new();
        let token_nonce = 0u64;
        let token_amount = BigUint::<StaticApi>::from(0u128);

        let response = self
            .interactor
            .tx()
            .from(&self.wallet_address)
            .to(self.state.current_address())
            .typed(proxy::SimpleLockEnergyProxy)
            .unlock_early()
            .payment((
                TokenIdentifier::from(token_id.as_str()),
                token_nonce,
                token_amount,
            ))
            .returns(ReturnsResultUnmanaged)
            .prepare_async()
            .run()
            .await;

        println!("Result: {response:?}");
    }

    async fn reduce_lock_period(&mut self) {
        let token_id = String::new();
        let token_nonce = 0u64;
        let token_amount = BigUint::<StaticApi>::from(0u128);

        let new_lock_period = 0u64;

        let response = self
            .interactor
            .tx()
            .from(&self.wallet_address)
            .to(self.state.current_address())
            .typed(proxy::SimpleLockEnergyProxy)
            .reduce_lock_period(new_lock_period)
            .payment((
                TokenIdentifier::from(token_id.as_str()),
                token_nonce,
                token_amount,
            ))
            .returns(ReturnsResultUnmanaged)
            .prepare_async()
            .run()
            .await;

        println!("Result: {response:?}");
    }

    async fn calculate_penalty_amount(&mut self) {
        let token_amount = BigUint::<StaticApi>::from(0u128);
        let prev_lock_epochs = 0u64;
        let new_lock_epochs = 0u64;

        let result_value = self
            .interactor
            .query()
            .to(self.state.current_address())
            .typed(proxy::SimpleLockEnergyProxy)
            .calculate_penalty_amount(token_amount, prev_lock_epochs, new_lock_epochs)
            .returns(ReturnsResultUnmanaged)
            .prepare_async()
            .run()
            .await;

        println!("Result: {result_value:?}");
    }

    async fn set_token_unstake_address(&mut self) {
        let sc_address = bech32::decode("");

        let response = self
            .interactor
            .tx()
            .from(&self.wallet_address)
            .to(self.state.current_address())
            .typed(proxy::SimpleLockEnergyProxy)
            .set_token_unstake_address(sc_address)
            .returns(ReturnsResultUnmanaged)
            .prepare_async()
            .run()
            .await;

        println!("Result: {response:?}");
    }

    // async fn revert_unstake(&mut self) {
    //     let token_id = String::new();
    //     let token_nonce = 0u64;
    //     let token_amount = BigUint::<StaticApi>::from(0u128);

    //     let user = bech32::decode("");
    //     let new_energy = PlaceholderInput;

    //     let response = self
    //         .interactor
    //         .tx()
    //         .from(&self.wallet_address)
    //         .to(self.state.current_address())
    //         .typed(proxy::SimpleLockEnergyProxy)
    //         .revert_unstake(user, new_energy)
    //         .payment((
    //             TokenIdentifier::from(token_id.as_str()),
    //             token_nonce,
    //             token_amount,
    //         ))
    //         .returns(ReturnsResultUnmanaged)
    //         .prepare_async()
    //         .run()
    //         .await;

    //     println!("Result: {response:?}");
    // }

    async fn token_unstake_sc_address(&mut self) {
        let result_value = self
            .interactor
            .query()
            .to(self.state.current_address())
            .typed(proxy::SimpleLockEnergyProxy)
            .token_unstake_sc_address()
            .returns(ReturnsResultUnmanaged)
            .prepare_async()
            .run()
            .await;

        println!("Result: {result_value:?}");
    }

    async fn set_energy_for_old_tokens(&mut self) {
        let users_energy = MultiValueVec::from(vec![MultiValue3::from((
            bech32::decode(""),
            BigUint::<StaticApi>::from(0u128),
            BigInt::<StaticApi>::from(0i64),
        ))]);

        let response = self
            .interactor
            .tx()
            .from(&self.wallet_address)
            .to(self.state.current_address())
            .typed(proxy::SimpleLockEnergyProxy)
            .set_energy_for_old_tokens(users_energy)
            .returns(ReturnsResultUnmanaged)
            .prepare_async()
            .run()
            .await;

        println!("Result: {response:?}");
    }

    // async fn update_energy_after_old_token_unlock(&mut self) {
    //     let original_caller = bech32::decode("");
    //     let initial_epoch_amount_pairs = PlaceholderInput;
    //     let final_epoch_amount_pairs = PlaceholderInput;

    //     let response = self
    //         .interactor
    //         .tx()
    //         .from(&self.wallet_address)
    //         .to(self.state.current_address())
    //         .typed(proxy::SimpleLockEnergyProxy)
    //         .update_energy_after_old_token_unlock(
    //             original_caller,
    //             initial_epoch_amount_pairs,
    //             final_epoch_amount_pairs,
    //         )
    //         .returns(ReturnsResultUnmanaged)
    //         .prepare_async()
    //         .run()
    //         .await;

    //     println!("Result: {response:?}");
    // }

    async fn migrate_old_tokens(&mut self) {
        let token_id = String::new();
        let token_nonce = 0u64;
        let token_amount = BigUint::<StaticApi>::from(0u128);

        let response = self
            .interactor
            .tx()
            .from(&self.wallet_address)
            .to(self.state.current_address())
            .typed(proxy::SimpleLockEnergyProxy)
            .migrate_old_tokens()
            .payment((
                TokenIdentifier::from(token_id.as_str()),
                token_nonce,
                token_amount,
            ))
            .returns(ReturnsResultUnmanaged)
            .prepare_async()
            .run()
            .await;

        println!("Result: {response:?}");
    }

    async fn pause_endpoint(&mut self) {
        let response = self
            .interactor
            .tx()
            .from(&self.wallet_address)
            .to(self.state.current_address())
            .typed(proxy::SimpleLockEnergyProxy)
            .pause_endpoint()
            .returns(ReturnsResultUnmanaged)
            .prepare_async()
            .run()
            .await;

        println!("Result: {response:?}");
    }

    async fn unpause_endpoint(&mut self) {
        let response = self
            .interactor
            .tx()
            .from(&self.wallet_address)
            .to(self.state.current_address())
            .typed(proxy::SimpleLockEnergyProxy)
            .unpause_endpoint()
            .returns(ReturnsResultUnmanaged)
            .prepare_async()
            .run()
            .await;

        println!("Result: {response:?}");
    }

    async fn paused_status(&mut self) {
        let result_value = self
            .interactor
            .query()
            .to(self.state.current_address())
            .typed(proxy::SimpleLockEnergyProxy)
            .paused_status()
            .returns(ReturnsResultUnmanaged)
            .prepare_async()
            .run()
            .await;

        println!("Result: {result_value:?}");
    }

    async fn set_transfer_role(&mut self, caller: &Bech32Address, address: &str) {
        let opt_address = OptionalValue::Some(bech32::decode(address));

        let response = self
            .interactor
            .tx()
            .from(caller)
            .gas(120_000_000u64)
            .to(self.state.current_address())
            .typed(proxy::SimpleLockEnergyProxy)
            .set_transfer_role(opt_address)
            .returns(ReturnsResultUnmanaged)
            .prepare_async()
            .run()
            .await;

        println!("Result: {response:?}");
    }

    async fn set_transfer_role_fail(
        &mut self,
        caller: &Bech32Address,
        address: &str,
        expected_result: ExpectError<'_>,
    ) {
        let opt_address = OptionalValue::Some(bech32::decode(address));

        let response = self
            .interactor
            .tx()
            .from(caller)
            .to(self.state.current_address())
            .typed(proxy::SimpleLockEnergyProxy)
            .set_transfer_role(opt_address)
            .returns(expected_result)
            .prepare_async()
            .run()
            .await;
    }

    async fn set_burn_role(&mut self, caller: &Bech32Address, address: &str) {
        let address = bech32::decode(address);

        let response = self
            .interactor
            .tx()
            .from(caller)
            .to(self.state.current_address())
            .typed(proxy::SimpleLockEnergyProxy)
            .set_burn_role(address)
            .returns(ReturnsResultUnmanaged)
            .prepare_async()
            .run()
            .await;

        println!("Result: {response:?}");
    }

    async fn set_burn_role_fail(
        &mut self,
        caller: &Bech32Address,
        address: &str,
        expected_result: ExpectError<'_>,
    ) {
        let address = bech32::decode(address);

        let response = self
            .interactor
            .tx()
            .from(caller)
            .to(self.state.current_address())
            .typed(proxy::SimpleLockEnergyProxy)
            .set_burn_role(address)
            .returns(expected_result)
            .prepare_async()
            .run()
            .await;

        println!("Result: {response:?}");
    }

    async fn merge_tokens_endpoint(&mut self) {
        let token_id = String::new();
        let token_nonce = 0u64;
        let token_amount = BigUint::<StaticApi>::from(0u128);

        let opt_original_caller = OptionalValue::Some(bech32::decode(""));

        let response = self
            .interactor
            .tx()
            .from(&self.wallet_address)
            .to(self.state.current_address())
            .typed(proxy::SimpleLockEnergyProxy)
            .merge_tokens_endpoint(opt_original_caller)
            .payment((
                TokenIdentifier::from(token_id.as_str()),
                token_nonce,
                token_amount,
            ))
            .returns(ReturnsResultUnmanaged)
            .prepare_async()
            .run()
            .await;

        println!("Result: {response:?}");
    }

    async fn lock_virtual(&mut self) {
        let token_id = TokenIdentifier::from_esdt_bytes(&b""[..]);
        let amount = BigUint::<StaticApi>::from(0u128);
        let lock_epochs = 0u64;
        let dest_address = bech32::decode("");
        let energy_address = bech32::decode("");

        let response = self
            .interactor
            .tx()
            .from(&self.wallet_address)
            .to(self.state.current_address())
            .typed(proxy::SimpleLockEnergyProxy)
            .lock_virtual(token_id, amount, lock_epochs, dest_address, energy_address)
            .returns(ReturnsResultUnmanaged)
            .prepare_async()
            .run()
            .await;

        println!("Result: {response:?}");
    }

    async fn add_sc_address_to_whitelist(&mut self) {
        let address = bech32::decode("");

        let response = self
            .interactor
            .tx()
            .from(&self.wallet_address)
            .to(self.state.current_address())
            .typed(proxy::SimpleLockEnergyProxy)
            .add_sc_address_to_whitelist(address)
            .returns(ReturnsResultUnmanaged)
            .prepare_async()
            .run()
            .await;

        println!("Result: {response:?}");
    }

    async fn remove_sc_address_from_whitelist(&mut self) {
        let address = bech32::decode("");

        let response = self
            .interactor
            .tx()
            .from(&self.wallet_address)
            .to(self.state.current_address())
            .typed(proxy::SimpleLockEnergyProxy)
            .remove_sc_address_from_whitelist(address)
            .returns(ReturnsResultUnmanaged)
            .prepare_async()
            .run()
            .await;

        println!("Result: {response:?}");
    }

    async fn is_sc_address_whitelisted(&mut self) {
        let address = bech32::decode("");

        let result_value = self
            .interactor
            .query()
            .to(self.state.current_address())
            .typed(proxy::SimpleLockEnergyProxy)
            .is_sc_address_whitelisted(address)
            .returns(ReturnsResultUnmanaged)
            .prepare_async()
            .run()
            .await;

        println!("Result: {result_value:?}");
    }

    async fn add_to_token_transfer_whitelist_fail(
        &mut self,
        caller: &Bech32Address,
        sc_addresses: Vec<Bech32Address>,
        expected_result: ExpectError<'_>,
    ) {
        let sc_addresses = MultiValueVec::from(sc_addresses);

        let response = self
            .interactor
            .tx()
            .from(caller)
            .to(self.state.current_address())
            .typed(proxy::SimpleLockEnergyProxy)
            .add_to_token_transfer_whitelist(sc_addresses)
            .returns(expected_result)
            .prepare_async()
            .run()
            .await;

        println!("Result: {response:?}");
    }

    async fn add_to_token_transfer_whitelist(
        &mut self,
        caller: &Bech32Address,
        sc_addresses: Vec<Bech32Address>,
    ) {
        let sc_addresses = MultiValueVec::from(sc_addresses);

        let response = self
            .interactor
            .tx()
            .from(caller)
            .to(self.state.current_address())
            .typed(proxy::SimpleLockEnergyProxy)
            .add_to_token_transfer_whitelist(sc_addresses)
            .returns(ReturnsResultUnmanaged)
            .prepare_async()
            .run()
            .await;

        println!("Result: {response:?}");
    }

    async fn remove_from_token_transfer_whitelist(&mut self) {
        let sc_addresses = MultiValueVec::from(vec![bech32::decode("")]);

        let response = self
            .interactor
            .tx()
            .from(&self.wallet_address)
            .to(self.state.current_address())
            .typed(proxy::SimpleLockEnergyProxy)
            .remove_from_token_transfer_whitelist(sc_addresses)
            .returns(ReturnsResultUnmanaged)
            .prepare_async()
            .run()
            .await;

        println!("Result: {response:?}");
    }

    async fn set_user_energy_after_locked_token_transfer(
        &mut self,
        user: &str,
        amount: i64,
        last_update_epoch: u64,
        total_locked_tokens: u128,
    ) {
        let user = bech32::decode(user);
        let energy = Energy {
            amount: BigInt::<StaticApi>::from(amount),
            last_update_epoch,
            total_locked_tokens: BigUint::<StaticApi>::from(total_locked_tokens),
        };

        let response = self
            .interactor
            .tx()
            .from(&self.wallet_address)
            .to(self.state.current_address())
            .typed(proxy::SimpleLockEnergyProxy)
            .set_user_energy_after_locked_token_transfer(user, energy)
            .returns(ReturnsResultUnmanaged)
            .prepare_async()
            .run()
            .await;

        println!("Result: {response:?}");
    }

    async fn set_user_energy_after_locked_token_transfer_fail(
        &mut self,
        user: &str,
        amount: i64,
        last_update_epoch: u64,
        total_locked_tokens: u128,
        expected_result: ExpectError<'_>,
    ) {
        let user = bech32::decode(user);
        let energy = Energy {
            amount: BigInt::<StaticApi>::from(amount),
            last_update_epoch,
            total_locked_tokens: BigUint::<StaticApi>::from(total_locked_tokens),
        };

        let response = self
            .interactor
            .tx()
            .from(&self.wallet_address)
            .to(self.state.current_address())
            .typed(proxy::SimpleLockEnergyProxy)
            .set_user_energy_after_locked_token_transfer(user, energy)
            .returns(expected_result)
            .prepare_async()
            .run()
            .await;

        println!("Result: {response:?}");
    }
}

#[tokio::test]
async fn test_deploy() {
    let mut interact = ContractInteract::new().await;
    interact
        .deploy(
            BASE_ASSET_TOKEN_ID,
            LEGACY_TOKEN_ID,
            OLD_LOCKED_ASSET_FACTORY_ADDRESS,
            180,
            vec![(360, 0), (361, 2_000), (362, 5_000)],
        )
        .await;
}

#[tokio::test]
async fn test_extend_lock_period_wrong_lock_time() {
    let mut interact = ContractInteract::new().await;
    interact.unpause_endpoint().await;
    interact
        .extend_lock_period_fail(
            &Bech32Address::from_bech32_string(ALICE_ADDRESS.to_string()),
            363,
            ALICE_ADDRESS,
            BASE_ASSET_TOKEN_ID,
            0,
            1,
            ExpectError(4, "Invalid lock choice"),
        )
        .await;
}

#[tokio::test]
async fn test_extend_lock_period_non_whitelisted() {
    let mut interact = ContractInteract::new().await;
    interact
        .extend_lock_period_fail(
            &Bech32Address::from_bech32_string(BOB_ADDRESS.to_string()),
            360,
            BOB_ADDRESS,
            BASE_ASSET_TOKEN_ID,
            0,
            1,
            ExpectError(4, "May not call this endpoint. Use lockTokens instead"),
        )
        .await;
}

#[tokio::test]
async fn test_lock_tokens_contract_paused() {
    let mut interact = ContractInteract::new().await;

    interact.pause_endpoint().await;

    interact
        .lock_tokens_endpoint_fail(
            360,
            ALICE_ADDRESS,
            BASE_ASSET_TOKEN_ID,
            0,
            1,
            ExpectError(4, "Contract is paused"),
        )
        .await;
}

#[tokio::test]
async fn test_lock_tokens_lock_not_listed() {
    let mut interact = ContractInteract::new().await;

    interact.unpause_endpoint().await;

    interact
        .lock_tokens_endpoint_fail(
            359,
            ALICE_ADDRESS,
            BASE_ASSET_TOKEN_ID,
            0,
            1,
            ExpectError(4, "Invalid lock choice"),
        )
        .await;
}

#[tokio::test]
async fn test_lock_tokens() {
    let mut interact = ContractInteract::new().await;
    interact.unpause_endpoint().await;
    interact
        .lock_tokens_endpoint(360, ALICE_ADDRESS, BASE_ASSET_TOKEN_ID, 0, 1)
        .await;
}

#[tokio::test]
async fn test_lock_tokens_wrong_lock_time() {
    let mut interact = ContractInteract::new().await;
    interact
        .lock_tokens_endpoint_fail(
            1,
            ALICE_ADDRESS,
            BASE_ASSET_TOKEN_ID,
            0,
            1,
            ExpectError(4, "Invalid lock choice"),
        )
        .await;
}

#[tokio::test]
async fn test_lock_tokens_wrong_token() {
    let mut interact = ContractInteract::new().await;
    interact
        .lock_tokens_endpoint_fail(
            360,
            ALICE_ADDRESS,
            ANOTHER_TOKEN_ID,
            0,
            1,
            ExpectError(4, "Invalid token ID"),
        )
        .await;
}

#[tokio::test]
async fn test_unlock_tokens_paused_contract() {
    let mut interact = ContractInteract::new().await;
    interact.pause_endpoint().await;
    interact
        .unlock_tokens_endpoint_fail(
            BASE_ASSET_TOKEN_ID,
            0,
            1,
            ExpectError(4, "Contract is paused"),
        )
        .await;
    interact.unpause_endpoint().await;
}

#[tokio::test]
async fn test_unlock_tokens_no_payments() {
    let mut interact = ContractInteract::new().await;
    interact.unlock_tokens_endpoint_no_payments().await;
}

#[tokio::test]
async fn test_unlock_tokens_diff_payments() {
    let mut interact = ContractInteract::new().await;
    interact.unlock_tokens_endpoint_diff_payments().await;
}

#[tokio::test]
async fn test_unlock_tokens_before_unlock_epoch() {
    let mut interact = ContractInteract::new().await;
    interact
        .unlock_tokens_endpoint_fail(
            BASE_ASSET_TOKEN_ID,
            0,
            1,
            ExpectError(4, "Cannot unlock yet"),
        )
        .await;
}

#[tokio::test]
async fn test_extend_lock_period_paused_contract() {
    let mut interact = ContractInteract::new().await;
    interact.pause_endpoint().await;
    interact
        .extend_lock_period_fail(
            &Bech32Address::from_bech32_string(ALICE_ADDRESS.to_string()),
            360,
            ALICE_ADDRESS,
            BASE_ASSET_TOKEN_ID,
            0,
            1,
            ExpectError(4, "Contract is paused"),
        )
        .await;
    interact.unpause_endpoint().await;
}

#[tokio::test]
async fn test_extend_lock_period_lock_not_listed() {
    let mut interact = ContractInteract::new().await;
    interact
        .extend_lock_period_fail(
            &Bech32Address::from_bech32_string(ALICE_ADDRESS.to_string()),
            1,
            ALICE_ADDRESS,
            BASE_ASSET_TOKEN_ID,
            0,
            1,
            ExpectError(4, "Invalid lock choice"),
        )
        .await;
}

#[tokio::test]
async fn test_extend_lock_not_whitelisted() {
    let mut interact = ContractInteract::new().await;
    interact
        .extend_lock_period_fail(
            &Bech32Address::from_bech32_string(ALICE_ADDRESS.to_string()),
            361,
            ALICE_ADDRESS,
            BASE_ASSET_TOKEN_ID,
            0,
            1,
            ExpectError(4, "May not call this endpoint. Use lockTokens instead"),
        )
        .await;
}

#[tokio::test]
async fn test_set_transfer_role_non_owner() {
    let mut interact = ContractInteract::new().await;
    interact
        .set_transfer_role_fail(
            &Bech32Address::from_bech32_string(BOB_ADDRESS.to_string()),
            BOB_ADDRESS,
            ExpectError(4, "Endpoint can only be called by owner"),
        )
        .await;
}

#[tokio::test]
async fn test_set_transfer_role_before_issue() {
    let mut interact = ContractInteract::new().await;

    interact
        .deploy(
            BASE_ASSET_TOKEN_ID,
            LEGACY_TOKEN_ID,
            OLD_LOCKED_ASSET_FACTORY_ADDRESS,
            180,
            vec![(360, 0), (361, 2_000), (362, 5_000)],
        )
        .await;

    interact
        .set_transfer_role_fail(
            &Bech32Address::from_bech32_string(ALICE_ADDRESS.to_string()),
            ALICE_ADDRESS,
            ExpectError(4, "Must issue or set token ID first"),
        )
        .await;
}

#[tokio::test]
async fn test_set_transfer_role() {
    let mut interact = ContractInteract::new().await;

    interact
        .issue_locked_token(1_000_000, "Mytoken", "ENG", 18)
        .await;

    interact
        .set_transfer_role(
            &Bech32Address::from_bech32_string(ALICE_ADDRESS.to_string()),
            ALICE_ADDRESS,
        )
        .await;
}

#[tokio::test]
async fn test_set_burn_role_non_owner() {
    let mut interact = ContractInteract::new().await;
    interact
        .set_burn_role_fail(
            &Bech32Address::from_bech32_string(BOB_ADDRESS.to_string()),
            BOB_ADDRESS,
            ExpectError(4, "Endpoint can only be called by owner"),
        )
        .await;
}

#[tokio::test]
async fn test_set_burn_role_before_issue() {
    let mut interact = ContractInteract::new().await;

    interact
        .deploy(
            BASE_ASSET_TOKEN_ID,
            LEGACY_TOKEN_ID,
            OLD_LOCKED_ASSET_FACTORY_ADDRESS,
            180,
            vec![(360, 0), (361, 2_000), (362, 5_000)],
        )
        .await;

    interact
        .set_burn_role_fail(
            &Bech32Address::from_bech32_string(ALICE_ADDRESS.to_string()),
            ALICE_ADDRESS,
            ExpectError(4, "Must issue or set token ID first"),
        )
        .await;
}

#[tokio::test]
async fn test_set_burn_role() {
    let mut interact = ContractInteract::new().await;

    interact
        .issue_locked_token(1_000_000, "Mytoken", "ENG", 18)
        .await;

    interact
        .set_burn_role(
            &Bech32Address::from_bech32_string(ALICE_ADDRESS.to_string()),
            ALICE_ADDRESS,
        )
        .await;
}

#[tokio::test]
async fn test_add_lock_options_too_many_options() {
    let mut interact = ContractInteract::new().await;

    interact
        .add_lock_options_fail(
            &Bech32Address::from_bech32_string(ALICE_ADDRESS.to_string()),
            vec![
                (360, 0),
                (361, 2_000),
                (362, 5_000),
                (363, 10_000),
                (364, 20_000),
                (365, 50_000),
                (366, 100_000),
                (366, 100_000),
                (366, 100_000),
                (366, 100_000),
                (366, 100_000),
            ],
            ExpectError(4, "Too many lock options"),
        )
        .await;
}

#[tokio::test]
async fn test_add_lock_options_non_admin() {
    let mut interact = ContractInteract::new().await;

    interact
        .add_lock_options_fail(
            &Bech32Address::from_bech32_string(BOB_ADDRESS.to_string()),
            vec![(360, 0), (361, 2_000), (362, 5_000)],
            ExpectError(4, "Endpoint can only be called by owner"),
        )
        .await;
}

#[tokio::test]
async fn test_add_lock_options_invalid_lock_epochs() {
    let mut interact = ContractInteract::new().await;

    interact
        .add_lock_options_fail(
            &Bech32Address::from_bech32_string(ALICE_ADDRESS.to_string()),
            vec![(359, 0), (361, 2_000), (362, 5_000)],
            ExpectError(4, "Invalid option"),
        )
        .await;
}

#[tokio::test]
async fn test_add_lock_options_invalid_penalty_percentage() {
    let mut interact = ContractInteract::new().await;

    interact
        .add_lock_options_fail(
            &Bech32Address::from_bech32_string(ALICE_ADDRESS.to_string()),
            vec![(365, 10_001), (361, 2_000), (362, 5_000)],
            ExpectError(4, "Invalid option"),
        )
        .await;
}

#[tokio::test]
async fn test_add_lock_options_duplicate_lock_options() {
    let mut interact = ContractInteract::new().await;

    interact
        .add_lock_options_fail(
            &Bech32Address::from_bech32_string(ALICE_ADDRESS.to_string()),
            vec![(365, 10_000), (370, 2_000), (365, 5_000)],
            ExpectError(4, "Duplicate lock options"),
        )
        .await;
}

#[tokio::test]
async fn test_add_lock_options_invalid_percentages() {
    let mut interact = ContractInteract::new().await;

    interact
        .add_lock_options_fail(
            &Bech32Address::from_bech32_string(ALICE_ADDRESS.to_string()),
            vec![(365, 6_000), (370, 7_000), (380, 5_500)],
            ExpectError(4, "Invalid lock option percentages"),
        )
        .await;

    interact
        .add_lock_options_fail(
            &Bech32Address::from_bech32_string(ALICE_ADDRESS.to_string()),
            vec![(365, 5_000)],
            ExpectError(4, "Invalid lock option percentages"),
        )
        .await;

    interact
        .add_lock_options_fail(
            &Bech32Address::from_bech32_string(ALICE_ADDRESS.to_string()),
            vec![(365, 2_000)],
            ExpectError(4, "Invalid lock option percentages"),
        )
        .await;
}

#[tokio::test]
async fn test_add_to_token_transfer_whitelist_non_owner() {
    let mut interact = ContractInteract::new().await;

    interact
        .add_to_token_transfer_whitelist_fail(
            &Bech32Address::from_bech32_string(BOB_ADDRESS.to_string()),
            vec![Bech32Address::from_bech32_string(ALICE_ADDRESS.to_string())],
            ExpectError(4, "Endpoint can only be called by owner"),
        )
        .await;
}

#[tokio::test]
async fn test_add_to_token_transfer_whitelist_non_sc_addresses() {
    let mut interact = ContractInteract::new().await;

    interact
        .add_to_token_transfer_whitelist_fail(
            &Bech32Address::from_bech32_string(ALICE_ADDRESS.to_string()),
            vec![Bech32Address::from_bech32_string(ALICE_ADDRESS.to_string())],
            ExpectError(4, "Invalid SC address"),
        )
        .await;
}

#[tokio::test]
async fn test_set_user_energy_after_locked_token_transfer_paused_contract() {
    let mut interact = ContractInteract::new().await;

    interact.pause_endpoint().await;

    interact
        .set_user_energy_after_locked_token_transfer_fail(
            ALICE_ADDRESS,
            0,
            0,
            0,
            ExpectError(4, "Contract is paused"),
        )
        .await;

    interact.unpause_endpoint().await;
}

#[tokio::test]
async fn test_set_user_energy_after_locked_token_transfer_non_whitelisted_caller() {
    let mut interact = ContractInteract::new().await;

    interact
        .set_user_energy_after_locked_token_transfer_fail(
            ALICE_ADDRESS,
            0,
            0,
            0,
            ExpectError(4, "Item not whitelisted"),
        )
        .await;
}

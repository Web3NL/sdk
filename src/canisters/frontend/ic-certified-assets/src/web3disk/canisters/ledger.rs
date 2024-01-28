use candid::{CandidType, Deserialize, Principal};
use ic_cdk::api::time;
use ic_cdk::id;
use ic_ledger_types::transfer;
use ic_ledger_types::{
    account_balance, AccountBalanceArgs, AccountIdentifier, BlockIndex, Memo, Subaccount,
    Timestamp, Tokens, TransferArgs, DEFAULT_SUBACCOUNT, MAINNET_CYCLES_MINTING_CANISTER_ID,
    MAINNET_LEDGER_CANISTER_ID,
};

pub static MEMO_TOP_UP_CANISTER: Memo = Memo(1347768404_u64);
pub static ICP_TRANSACTION_FEE: Tokens = Tokens::from_e8s(10000);

/*
Get the default account of this canister and its ICP balance
*/
pub async fn get_default_account_and_balance() -> DefaultAccountAndBalance {
    let account = AccountIdentifier::new(&id(), &DEFAULT_SUBACCOUNT).to_string();
    let balance: Tokens = icp_balance().await;

    let e8s: u64 = balance.e8s();
    let e3s: u64 = e8s / 100_000;

    let balance: f64 = e3s as f64 / 1000.;

    DefaultAccountAndBalance { account, balance }
}

#[derive(CandidType, Deserialize)]
pub struct DefaultAccountAndBalance {
    account: String,
    balance: f64,
}

/*
Query the ledger canister for the icp balance of the default account of this canister
*/
pub async fn icp_balance() -> Tokens {
    let arg = AccountBalanceArgs {
        account: AccountIdentifier::new(&id(), &DEFAULT_SUBACCOUNT),
    };

    let balance = account_balance(MAINNET_LEDGER_CANISTER_ID, arg)
        .await
        .expect("Failed to query ledger canister for balance");

    balance
}

/*
Transfer ICP to CMC canister for cycle minting
*/
pub async fn transfer_icp_to_cmc_for_cycles_minting(
    amount: Tokens,
    canister_id: Principal,
) -> BlockIndex {
    let arg = TransferArgs {
        memo: MEMO_TOP_UP_CANISTER,
        amount,
        fee: ICP_TRANSACTION_FEE,
        from_subaccount: None,
        to: AccountIdentifier::new(
            &MAINNET_CYCLES_MINTING_CANISTER_ID,
            &Subaccount::from(canister_id),
        ),
        created_at_time: Some(Timestamp {
            timestamp_nanos: time(),
        }),
    };

    transfer(MAINNET_LEDGER_CANISTER_ID, arg)
        .await
        .expect("Failed to transfer ICP to CMC")
        .unwrap_or_else(|err| {
            ic_cdk::trap(&format!(
                "Failed to transfer ICP to CMC for cycle minting: {:?}",
                err
            ))
        })
}

use candid::{CandidType, Deserialize, Principal};
use ic_cdk::call;
use ic_ledger_types::{BlockIndex, MAINNET_CYCLES_MINTING_CANISTER_ID};

/*
Query the CMC canister for the ICP/XDR rate
*/
pub async fn icp_xdr_rate() -> IcpXdrConversionRate {
    call::<(), (IcpXdrConversionRateResponse,)>(
        MAINNET_CYCLES_MINTING_CANISTER_ID,
        "get_icp_xdr_conversion_rate",
        (),
    )
    .await
    .expect("Failed to call get_icp_to_xdr_rate")
    .0
    .data
}

#[derive(CandidType, Deserialize)]
pub struct IcpXdrConversionRateResponse {
    certificate: Vec<u8>,
    data: IcpXdrConversionRate,
    hash_tree: Vec<u8>,
}

#[derive(CandidType, Deserialize)]
pub struct IcpXdrConversionRate {
    pub xdr_permyriad_per_icp: u64,
    timestamp_seconds: u64,
}

/*
Notify CMC of a ICP tx for cycle minting
*/
pub async fn notify_top_up(block_index: BlockIndex, canister_id: Principal) -> Cycles {
    let arg = NotifyTopUpArg {
        block_index,
        canister_id,
    };

    let notify_top_ip_result = call::<(NotifyTopUpArg,), (NotifyTopUpResult,)>(
        MAINNET_CYCLES_MINTING_CANISTER_ID,
        "notify_top_up",
        (arg,),
    )
    .await
    .expect("Failed to call notify_top_up")
    .0;

    match notify_top_ip_result {
        NotifyTopUpResult::Ok(cycles) => cycles,
        NotifyTopUpResult::Err(err) => {
            ic_cdk::trap(&format!("Failed to notify_top_up: {:?}", err));
        }
    }
}

#[derive(CandidType, Deserialize)]
pub struct NotifyTopUpArg {
    pub block_index: BlockIndex,
    pub canister_id: Principal,
}

pub type Cycles = candid::Nat;
#[derive(CandidType, Deserialize)]
pub enum NotifyTopUpResult {
    Ok(Cycles),
    Err(NotifyError),
}

#[derive(CandidType, Deserialize, Debug)]
pub enum NotifyError {
    Refunded {
        block_index: Option<BlockIndex>,
        reason: String,
    },
    InvalidTransaction(String),
    Other {
        error_message: String,
        error_code: u64,
    },
    Processing,
    TransactionTooOld(BlockIndex),
}

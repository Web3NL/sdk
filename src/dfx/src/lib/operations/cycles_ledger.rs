use std::time::{SystemTime, UNIX_EPOCH};

use crate::lib::cycles_ledger_types;
use crate::lib::cycles_ledger_types::send::SendError;
use crate::lib::environment::Environment;
use crate::lib::error::DfxResult;
use crate::lib::ic_attributes::CanisterSettings as DfxCanisterSettings;
use crate::lib::operations::canister::create_canister::{
    CANISTER_CREATE_FEE, CANISTER_INITIAL_CYCLE_BALANCE,
};
use crate::lib::retryable::retryable;
use anyhow::{anyhow, bail};
use backoff::future::retry;
use backoff::ExponentialBackoff;
use candid::{CandidType, Decode, Encode, Nat, Principal};
use fn_error_context::context;
use ic_agent::Agent;
use ic_utils::call::SyncCall;
use ic_utils::interfaces::management_canister::builders::CanisterSettings;
use ic_utils::Canister;
use icrc_ledger_types::icrc1;
use icrc_ledger_types::icrc1::account::Subaccount;
use icrc_ledger_types::icrc1::transfer::{BlockIndex, TransferError};
use serde::Deserialize;
use slog::{info, Logger};
use thiserror::Error;

/// Cycles ledger feature flag to turn off behavior that would be confusing while cycles ledger is not enabled yet.
//TODO(SDK-1331): feature flag can be removed
pub const CYCLES_LEDGER_ENABLED: bool = false;

const ICRC1_BALANCE_OF_METHOD: &str = "icrc1_balance_of";
const ICRC1_TRANSFER_METHOD: &str = "icrc1_transfer";
const SEND_METHOD: &str = "send";
const CREATE_CANISTER_METHOD: &str = "create_canister";
const CYCLES_LEDGER_CANISTER_ID: Principal =
    Principal::from_slice(&[0x00, 0x00, 0x00, 0x00, 0x02, 0x10, 0x00, 0x02, 0x01, 0x01]);

pub async fn balance(
    agent: &Agent,
    owner: Principal,
    subaccount: Option<icrc1::account::Subaccount>,
) -> DfxResult<u128> {
    let canister = Canister::builder()
        .with_agent(agent)
        .with_canister_id(CYCLES_LEDGER_CANISTER_ID)
        .build()?;
    let arg = icrc1::account::Account { owner, subaccount };

    let retry_policy = ExponentialBackoff::default();

    retry(retry_policy, || async {
        let result = canister
            .query(ICRC1_BALANCE_OF_METHOD)
            .with_arg(arg)
            .build()
            .call()
            .await;
        match result {
            Ok((balance,)) => Ok(balance),
            Err(agent_err) if retryable(&agent_err) => {
                Err(backoff::Error::transient(anyhow!(agent_err)))
            }
            Err(agent_err) => Err(backoff::Error::permanent(anyhow!(agent_err))),
        }
    })
    .await
}

pub async fn transfer(
    agent: &Agent,
    logger: &Logger,
    amount: u128,
    from_subaccount: Option<icrc1::account::Subaccount>,
    owner: Principal,
    to_subaccount: Option<icrc1::account::Subaccount>,
    created_at_time: u64,
    memo: Option<u64>,
) -> DfxResult<BlockIndex> {
    let canister = Canister::builder()
        .with_agent(agent)
        .with_canister_id(CYCLES_LEDGER_CANISTER_ID)
        .build()?;

    let retry_policy = ExponentialBackoff::default();

    let block_index = retry(retry_policy, || async {
        let arg = icrc1::transfer::TransferArg {
            from_subaccount,
            to: icrc1::account::Account {
                owner,
                subaccount: to_subaccount,
            },
            fee: None,
            created_at_time: Some(created_at_time),
            memo: memo.map(|v| v.into()),
            amount: Nat::from(amount),
        };
        match canister
            .update(ICRC1_TRANSFER_METHOD)
            .with_arg(arg)
            .build()
            .map(|result: (Result<BlockIndex, TransferError>,)| (result.0,))
            .call_and_wait()
            .await
            .map(|(result,)| result)
        {
            Ok(Ok(block_index)) => Ok(block_index),
            Ok(Err(TransferError::Duplicate { duplicate_of })) => {
                info!(
                    logger,
                    "{}",
                    TransferError::Duplicate {
                        duplicate_of: duplicate_of.clone()
                    }
                );
                Ok(duplicate_of)
            }
            Ok(Err(transfer_err)) => Err(backoff::Error::permanent(anyhow!(transfer_err))),
            Err(agent_err) if retryable(&agent_err) => {
                Err(backoff::Error::transient(anyhow!(agent_err)))
            }
            Err(agent_err) => Err(backoff::Error::permanent(anyhow!(agent_err))),
        }
    })
    .await?;

    Ok(block_index)
}

pub async fn send(
    agent: &Agent,
    logger: &Logger,
    to: Principal,
    amount: u128,
    created_at_time: u64,
    from_subaccount: Option<icrc1::account::Subaccount>,
) -> DfxResult<BlockIndex> {
    let canister = Canister::builder()
        .with_agent(agent)
        .with_canister_id(CYCLES_LEDGER_CANISTER_ID)
        .build()?;

    let retry_policy = ExponentialBackoff::default();
    let block_index: BlockIndex = retry(retry_policy, || async {
        let arg = cycles_ledger_types::send::SendArgs {
            from_subaccount,
            to,
            created_at_time: Some(created_at_time),
            amount: Nat::from(amount),
        };
        match canister
            .update(SEND_METHOD)
            .with_arg(arg)
            .build()
            .map(|result: (Result<BlockIndex, SendError>,)| (result.0,))
            .call_and_wait()
            .await
            .map(|(result,)| result)
        {
            Ok(Ok(block_index)) => Ok(block_index),
            Ok(Err(SendError::Duplicate { duplicate_of })) => {
                info!(
                    logger,
                    "transaction is a duplicate of another transaction in block {}", duplicate_of
                );
                Ok(duplicate_of)
            }
            Ok(Err(SendError::InvalidReceiver { receiver })) => {
                Err(backoff::Error::permanent(anyhow!(
                    "Invalid receiver: {}.  Make sure the receiver is a canister.",
                    receiver
                )))
            }
            Ok(Err(send_err)) => Err(backoff::Error::permanent(anyhow!(
                "send error: {:?}",
                send_err
            ))),
            Err(agent_err) if retryable(&agent_err) => {
                Err(backoff::Error::transient(anyhow!(agent_err)))
            }
            Err(agent_err) => Err(backoff::Error::permanent(anyhow!(agent_err))),
        }
    })
    .await?;

    Ok(block_index)
}

#[context("Failed to create canister via cycles ledger.")]
pub async fn create_with_cycles_ledger(
    env: &dyn Environment,
    agent: &Agent,
    canister_name: &str,
    with_cycles: Option<u128>,
    from_subaccount: Option<Subaccount>,
    settings: DfxCanisterSettings,
    created_at_time: Option<u64>,
) -> DfxResult<Principal> {
    #[derive(CandidType, Clone, Debug)]
    // TODO(FI-1022): Import types from cycles ledger crate once available
    struct CreateCanisterArgs {
        pub from_subaccount: Option<icrc_ledger_types::icrc1::account::Subaccount>,
        pub created_at_time: Option<u64>,
        pub amount: u128,
        pub creation_args: Option<CmcCreateCanisterArgs>,
    }
    #[derive(CandidType, Clone, Debug)]
    struct CmcCreateCanisterArgs {
        pub subnet_selection: Option<SubnetSelection>,
        pub settings: Option<CanisterSettings>,
    }
    #[derive(CandidType, Clone, Debug)]
    #[allow(dead_code)]
    enum SubnetSelection {
        /// Choose a random subnet that satisfies the specified properties
        Filter(SubnetFilter),
        /// Choose a specific subnet
        Subnet { subnet: Principal },
    }
    #[derive(CandidType, Clone, Debug)]
    struct SubnetFilter {
        pub subnet_type: Option<String>,
    }
    #[derive(CandidType, Clone, Debug, Deserialize, Error)]
    enum CreateCanisterError {
        #[error("Insufficient funds. Current balance: {balance}")]
        InsufficientFunds { balance: u128 },
        #[error("Local clock too far behind.")]
        TooOld,
        #[error("Local clock too far ahead.")]
        CreatedInFuture { ledger_time: u64 },
        #[error("Cycles ledger temporarily unavailable.")]
        TemporarilyUnavailable,
        #[error("Duplicate of block {duplicate_of}.")]
        Duplicate {
            duplicate_of: Nat,
            canister_id: Option<Principal>,
        },
        #[error("Cycles ledger failed to create canister: {error}")]
        FailedToCreate {
            fee_block: Option<Nat>,
            refund_block: Option<Nat>,
            error: String,
        },
        #[error("Ledger error {error_code}: {message}")]
        GenericError { error_code: Nat, message: String },
    }
    #[derive(Deserialize, CandidType, Clone, Debug, PartialEq, Eq)]
    struct CreateCanisterSuccess {
        pub block_id: Nat,
        pub canister_id: Principal,
    }

    let cycles = with_cycles.unwrap_or(CANISTER_CREATE_FEE + CANISTER_INITIAL_CYCLE_BALANCE);
    let created_at_time = created_at_time.or_else(|| {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos() as u64;
        info!(
            env.get_logger(),
            "created-at-time for canister {canister_name} is {now}."
        );
        Some(now)
    });

    let result = loop {
        match agent
            .update(&CYCLES_LEDGER_CANISTER_ID, CREATE_CANISTER_METHOD)
            .with_arg(
                Encode!(&CreateCanisterArgs {
                    from_subaccount,
                    created_at_time,
                    amount: cycles,
                    creation_args: Some(CmcCreateCanisterArgs {
                        settings: Some(settings.clone().into()),
                        subnet_selection: None,
                    }),
                })
                .unwrap(),
            )
            .call_and_wait()
            .await
        {
            Ok(result) => break result,
            Err(err) => {
                if retryable(&err) {
                    info!(env.get_logger(), "Request error: {err:?}. Retrying...");
                } else {
                    bail!(err)
                }
            }
        }
    };
    let create_result = Decode!(
        &result,
        Result<CreateCanisterSuccess, CreateCanisterError>
    )
    .map_err(|err| {
        anyhow!(
            "Failed to decode cycles ledger response: {}",
            err.to_string()
        )
    })?;
    match create_result {
        Ok(result) => Ok(result.canister_id),
        Err(CreateCanisterError::Duplicate {
            duplicate_of,
            canister_id,
        }) => {
            if let Some(canister) = canister_id {
                info!(env.get_logger(), "Duplicate of block {duplicate_of}. Canister already created with id {canister}.");
                Ok(canister)
            } else {
                bail!("Duplicate of block {duplicate_of} but no canister id is available.");
            }
        }
        Err(err) => bail!(err),
    }
}

#[test]
fn ledger_canister_id_text_representation() {
    assert_eq!(
        Principal::from_text("um5iw-rqaaa-aaaaq-qaaba-cai").unwrap(),
        CYCLES_LEDGER_CANISTER_ID
    );
}

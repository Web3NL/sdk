use candid::CandidType;
use ic_cdk::{
    api::management_canister::{
        main::{canister_status, CanisterStatusResponse},
        provisional::CanisterIdRecord,
    },
    trap,
};
use num_bigint::BigUint;
use num_traits::ToPrimitive;

#[derive(Clone, CandidType)]
pub struct CanisterInfo {
    // T cycles
    pub cycles: f64,
    // memory size in MB
    pub memory: f64,
}

impl From<CanisterStatusResponse> for CanisterInfo {
    fn from(canister_status: CanisterStatusResponse) -> Self {
        /*
        * Convert cycle balance in candid::NAT to f64
        */
        let cycles: BigUint = canister_status.cycles.0;

        // 1 T cycles = 10^12 cycles
        // Divide by 10^9 to get 10^3 T cycles (whole number)
        let cycles: BigUint = cycles / 1_000_000_000 as u64;
        let cycles: f64 = cycles
            .to_f64()
            .unwrap_or_else(|| trap("Failed to convert BigUint to f64"));

        // Divide by 10^3 to get T cycles with three decimals
        let cycles: f64 = cycles  / 1_000.;

        /*
        * Convert memory_size in bytes candid::Nat to MB f64
        */
        let memory: BigUint = canister_status.memory_size.0;

        // 1 MB = 10^6 bytes
        // Divide by 10^3 to get kB (whole number)
        let memory: BigUint = memory / 1000 as u64;
        let memory: f64 = memory
            .to_f64()
            .unwrap_or_else(|| trap("Failed to convert BigUint to f64"));

        // Divide by 10^3 to get MB with three decimals
        let memory: f64 = memory / 1000 as f64;

        Self { cycles, memory}
    }
}

pub async fn _settings_info() -> CanisterInfo {
    let arg = CanisterIdRecord {
        canister_id: ic_cdk::api::id(),
    };

    let canister_status_response = canister_status(arg)
        .await
        .unwrap_or_else(|err| trap(&format!("{:?}", err)))
        .0;

    CanisterInfo::from(canister_status_response)
}

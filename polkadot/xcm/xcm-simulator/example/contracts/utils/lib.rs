#![cfg_attr(not(feature = "std"), no_std, no_main)]

pub use sp_weights::Weight;
use ink::prelude::boxed::Box;
use ink::prelude::{vec, vec::Vec};
use ink::primitives::AccountId;
use xcm::v3::prelude::*;

use sp_weights::constants::{WEIGHT_REF_TIME_PER_SECOND, WEIGHT_PROOF_SIZE_PER_MB};

const WEIGHT_PER_INSTRUCTION: Weight = Weight::from_parts(1, 1);
const UNITS_PER_SECOND: u128 = 1;
const UNITS_PER_MB: u128 = 1;

pub fn estimate_message_fee(number_of_instructions: u64) -> u128 {
    let weight = estimate_weight(number_of_instructions);
    estimate_fee_for_weight(weight)
}

pub fn estimate_weight(number_of_instructions: u64) -> Weight {
    WEIGHT_PER_INSTRUCTION.saturating_mul(number_of_instructions)
}

pub fn estimate_fee_for_weight(weight: Weight) -> u128 {
    UNITS_PER_SECOND * (weight.ref_time() as u128) / (WEIGHT_REF_TIME_PER_SECOND as u128)
        + UNITS_PER_MB * (weight.proof_size() as u128) / (WEIGHT_PROOF_SIZE_PER_MB as u128)
}

pub fn make_xcm_contract_call<C: ink::env::ContractEnv>(
    path_to_chain: xcm::VersionedMultiLocation,
    contract_address: AccountId,
    payload: Vec<u8>,
    value: u128,
    gas_limit: Option<Weight>,
) -> Result<(), ink::env::Error> {
    let gas_limit = gas_limit.unwrap_or(Weight::from_all(1_000_000_000_000));
    // let est_wt = estimate_weight(4) + gas_limit * 2;
    // let fee = estimate_fee_for_weight(est_wt);

    let contract_call = RuntimeCall::Contracts(ContractsCall::Call {
        dest: contract_address,
        value,
        gas_limit,
        storage_deposit_limit: None,
        data: payload,
    });

    let message: Xcm<()> = Xcm(vec![
        // WithdrawAsset(vec![(Parent, fee).into()].into()),
        // BuyExecution {
        //     fees: (Parent, fee).into(),
        //     weight_limit: WeightLimit::Unlimited,
        // },
        UnpaidExecution {
            weight_limit: WeightLimit::Unlimited,
            check_origin: None,  
        },
        Transact {
            origin_kind: OriginKind::SovereignAccount,
            require_weight_at_most: gas_limit * 2,
            call: scale::Encode::encode(&contract_call).into(),
        },
    ]);

    let xcm_call = RuntimeCall::Xcm(XcmCall::Send {
        dest: Box::new(path_to_chain),
        message: Box::new(xcm::VersionedXcm::V3(message)),
    });

    ink::env::call_runtime::<C::Env, _>(&xcm_call)
}

// @dev Make sure indexes are valid for chain in use!
#[derive(scale::Encode)]
pub enum RuntimeCall {
    #[codec(index = 70)]
    Contracts(ContractsCall),
    #[codec(index = 99)]
    Xcm(XcmCall),
}

#[derive(scale::Encode)]
pub enum XcmCall {
    #[codec(index = 0)]
    Send {
        dest: Box<xcm::VersionedMultiLocation>,
        message: Box<xcm::VersionedXcm<()>>,
    },
}

#[derive(scale::Encode)]
pub enum ContractsCall {
    #[codec(index = 6)]
    Call {
        dest: AccountId,
        #[codec(compact)]
        value: u128,
        gas_limit: crate::Weight,
        storage_deposit_limit: Option<u128>,
        data: Vec<u8>,
    },
}

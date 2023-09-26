use super::*;
pub use sp_core::{hexdisplay::AsBytesRef, Bytes};
use sp_runtime::AccountId32;
pub use std::str::FromStr;
pub use xcm::v3::prelude::*;
pub use xcm_simulator::TestExt;
pub use codec::{Decode, Encode};
pub use frame_support::assert_ok;
pub use crate::parachain::Balance;

pub const TX_GAS: u64 = 10_000_000_000;

fn encode_selector(sel: &str) -> [u8; 4] {
    let bytes = Bytes::from_str(sel).unwrap().0;
    [bytes[0], bytes[1], bytes[2], bytes[3]]
}

pub fn deploy_contract(blob: Vec<u8>, sel_constr: Vec<u8>, deployer: AccountId32) -> AccountId32 {
    let resp = ParachainContracts::bare_instantiate(
        deployer,
        0,
        TX_GAS.into(),
        None,
        pallet_contracts_primitives::Code::Upload(blob),
        sel_constr,
        vec![],
        pallet_contracts::DebugInfo::UnsafeDebug,
        pallet_contracts::CollectEvents::UnsafeCollect,
    );

    resp.result.expect("Failed to init contract").account_id
}

pub fn call_contract(
    contract: &AccountId32,
    caller: AccountId32,
    msg: Vec<u8>,
    value: Balance,
) -> Vec<u8> {
    let rs = ParachainContracts::bare_call(
        caller,
        contract.clone(),
        value,
        TX_GAS.into(),
        None,
        msg,
        pallet_contracts::DebugInfo::UnsafeDebug,
        pallet_contracts::CollectEvents::UnsafeCollect,
        pallet_contracts::Determinism::Enforced,
    )
    .result
    .expect("execution without result");

    let pallet_contracts_primitives::ExecReturnValue { flags: _, mut data } = rs;

    // InkLang error check
    assert_eq!(data.remove(0), 0);

    data
}

fn setup() -> (AccountId32, AccountId32) {
    // 1. Deploy demo contract on chain A
    let demo = ParaA::execute_with(|| {
        let blob =
            std::fs::read("./contracts/target/ink/demo/demo.wasm").expect("cound not find wasm blob");

        let sel_constructor = encode_selector("0x9bae9d5e");
        let payload = sel_constructor.encode();

        deploy_contract(blob, payload, ALICE)
    });

    // 2. Deploy xc-demo on chain B
    let xc_demo = ParaB::execute_with(|| {
        let blob =
            std::fs::read("./contracts/target/ink/xc_demo/xc_demo.wasm").expect("cound not find wasm blob");

        let sel_constructor = encode_selector("0x9bae9d5e");
        let payload = (sel_constructor, demo.clone()).encode();

        deploy_contract(blob, payload, ALICE)
    });

    (demo, xc_demo)
}

#[test]
fn xcm_call_runtime_bug() {
    // 0. Deploy contracts
    let (demo, xc_demo) = setup();

    // 1. Call xc_demo::call_demo
    ParaB::execute_with(|| {
        let sel_call_demo = encode_selector("0x492e633b");
        let payload = sel_call_demo.encode();

        let encoded_data = call_contract(&xc_demo, ALICE, payload, 0);
        let data: Result<(), u8> = Decode::decode(&mut &encoded_data[..]).expect("failed to decode");

        assert_eq!(data, Ok(()));
    });

    // 2. Verify demo::get_demo_count == 1
    let value_in_demo = ParaA::execute_with(|| {
        let sel_get_demo_count = encode_selector("0x07c7c213");
        let payload = sel_get_demo_count.encode();

        let encoded_data = call_contract(&demo, ALICE, payload, 0);
        let data: u128 = Decode::decode(&mut &encoded_data[..]).expect("failed to decode");
    
        data
    });
    assert_eq!(value_in_demo, 1);

    // 3. Verify xc_demo::get_demo_count == 1
    let value_in_xc_demo = ParaB::execute_with(|| {
        let sel_get_demo_count = encode_selector("0x07c7c213");
        let payload = sel_get_demo_count.encode();

        let encoded_data = call_contract(&xc_demo, ALICE, payload, 0);
        let data: u128 = Decode::decode(&mut &encoded_data[..]).expect("failed to decode");
    
        data
    });
    assert_eq!(value_in_xc_demo, 1);
}

// use std::sync::Once;
// static INIT: Once = Once::new();
// fn init_tracing() {
//     INIT.call_once(|| {
//         // Add test tracing (from sp_tracing::init_for_tests()) but filtering for xcm logs only
//         tracing_subscriber::fmt()
//             .with_max_level(tracing::Level::TRACE)
//             .with_env_filter("xcm=trace,system::events=trace,runtime::contracts=debug") // Comment out this line to see all traces
//             .with_test_writer()
//             .init();
//     });
// }
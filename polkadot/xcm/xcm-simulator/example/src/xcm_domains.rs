use super::*;
use xcm_call_runtime::*;
use sp_runtime::AccountId32;

fn encode_selector(sel: &str) -> [u8; 4] {
    let bytes = Bytes::from_str(sel).unwrap().0;
    [bytes[0], bytes[1], bytes[2], bytes[3]]
}

fn deploy_state_manager() -> AccountId32 {
    let blob =
        std::fs::read("./contracts/target/ink/who_am_i/who_am_i.wasm").expect("cound not find wasm blob");

    let sel_constructor = encode_selector("0x9bae9d5e");
    let payload = (sel_constructor, ALICE, ALICE).encode(); // (selector, admin, handler)

    deploy_contract(blob, payload, ALICE)
}

fn deploy_xcm_handler(state_manager: &AccountId32) -> AccountId32 {
    let blob = std::fs::read("./contracts/target/ink/handler_who_am_i/handler_who_am_i.wasm")
        .expect("cound not find wasm blob");

    let sel_constructor = encode_selector("0x9bae9d5e");
    let payload = (sel_constructor, ALICE, state_manager).encode(); // (selector, admin, state_manager)

    deploy_contract(blob, payload, ALICE)
}

fn deploy_xc_contract(xcm_handler: &AccountId32, xcm_handler_soac: &AccountId32) -> AccountId32 {
    let blob = std::fs::read("./contracts/target/ink/xc_who_am_i/xc_who_am_i.wasm")
        .expect("cound not find wasm blob");

    let sel_constructor = encode_selector("0x9bae9d5e");
    let payload = (sel_constructor, xcm_handler, xcm_handler_soac).encode(); // (selector, xcm_handler, xcm_handler_sovereign_account)

    deploy_contract(blob, payload, ALICE)
}

fn set_handler(state_manager: &AccountId32, xcm_handler: &AccountId32) {
    let sel_set_handler = encode_selector("0xee45cea1");
    let payload = (sel_set_handler, xcm_handler).encode();

    let encoded_resp = call_contract(state_manager, ALICE, payload, 0);
    let resp: Result<(), u8> = Decode::decode(&mut &encoded_resp[..]).expect("failed to decode");

    assert_eq!(resp, Ok(()));
}

fn add_xc_contract(
    xcm_handler: &AccountId32,
    xc_contract_soac: &AccountId32,
    location: &(u32, AccountId32),
) {
    let sel_add_xc_contract = encode_selector("0x5578fb41");
    let payload = (sel_add_xc_contract, xc_contract_soac, location).encode();

    let encoded_resp = call_contract(xcm_handler, ALICE, payload, 0);
    let resp: Result<(), u8> = Decode::decode(&mut &encoded_resp[..]).expect("failed to decode");

    assert_eq!(resp, Ok(()));
}

fn verify_contract_state(contract: &AccountId32, id: u128, user: Option<AccountId32>) {
    let get_counter = || {
        let sel_counter = Bytes::from_str("0x94fc951c")
            .map(|v| v.to_vec())
            .expect("unable to parse hex string");

        let data = call_contract(contract, ALICE, sel_counter, 0);
        u128::decode(&mut data.as_bytes_ref()).expect("failed to decode")
    };

    let get_last_visitor = || {
        let sel_last_visitor = Bytes::from_str("0xef8c2bd9")
            .map(|v| v.to_vec())
            .expect("unable to parse hex string");

        let data = call_contract(contract, ALICE, sel_last_visitor, 0);
        Option::<AccountId32>::decode(&mut data.as_bytes_ref()).expect("failed to decode")
    };

    let find_who_am_i = |id: u128| {
        let mut sel_who_am_i = Bytes::from_str("0x8fb9cb05")
            .map(|v| v.to_vec())
            .expect("unable to parse hex string");
        sel_who_am_i.append(&mut id.encode());

        let data = call_contract(contract, ALICE, sel_who_am_i, 0);
        Option::<AccountId32>::decode(&mut data.as_bytes_ref()).expect("failed to decode")
    };

    ParaA::execute_with(|| {
        assert_eq!(get_counter(), id);
        assert_eq!(get_last_visitor(), user);
        assert_eq!(find_who_am_i(id), user);
    });
}

// fn fund_address(addr: &AccountId32) {
//     assert_ok!(ParachainBalances::force_set_balance(
//         parachain::RuntimeOrigin::root(),
//         addr.clone(),
//         INITIAL_BALANCE,
//     ));

//     assert_ok!(ParachainAssets::mint(
//         parachain::RuntimeOrigin::signed(ADMIN),
//         0,
//         addr.clone(),
//         INITIAL_BALANCE
//     ));
// }

fn setup() -> (AccountId32, AccountId32, AccountId32) {
    // 1. Deploy `who_am_i`
    let state_manager = ParaA::execute_with(deploy_state_manager);
    println!("state_manager: {:?}", state_manager);

    // 2A. Deploy `handler_who_am_i`
    let xcm_handler = ParaA::execute_with(|| deploy_xcm_handler(&state_manager));
    let xcm_handler_soac = sibling_account_account_id(1, xcm_handler.clone());
    println!("xcm_handler: {:?}", xcm_handler);

    // 2B. Update state_manager::handler
    ParaA::execute_with(|| set_handler(&state_manager, &xcm_handler));

    // 3A. Deploy `xc_who_am_i`
    let xc_contract = ParaB::execute_with(|| deploy_xc_contract(&xcm_handler, &xcm_handler_soac));
    let xc_contract_soac = sibling_account_account_id(2, xc_contract.clone());
    println!("xc_contract: {:?}", xc_contract);

    // 3B. Approve xc_contract on xcm_handler
    let location = (2, xc_contract.clone());
    ParaA::execute_with(|| add_xc_contract(&xcm_handler, &xc_contract_soac, &location));

    // 4. Fund sovereign accounts for gas fee payment
    // ParaB::execute_with(|| fund_address(&xcm_handler_soac));
    // ParaA::execute_with(|| fund_address(&xc_contract_soac));

    (state_manager, xcm_handler, xc_contract)
}

#[test]
fn setup_works() {
    MockNet::reset();
    setup();
}

#[test]
fn callback_works() {
    init_tracing();
    MockNet::reset();

    let (state_manager, xcm_handler, xc_contract) = setup();

    // ParaA::execute_with(|| {
    //     let xc_contract_soac = sibling_account_account_id(2, xc_contract.clone());

    //     assert_ok!(ParachainBalances::force_set_balance(
    //         parachain::RuntimeOrigin::root(),
    //         xc_contract_soac.clone(),
    //         INITIAL_BALANCE,
    //     ));

    //     // let sel_walk_in = encode_selector("0xc0397d90");
    //     // let payload = (sel_walk_in, ALICE).encode();
    //     // call_contract(&xcm_handler, BOB, payload, 0);
    // });

    // Relay::execute_with(|| {
    //     assert_ok!(RelayChainUniques::mint(
    //         relay_chain::RuntimeOrigin::signed(ALICE), 
    //         1, 
    //         43, 
    //         child_account_id(2)
    //     )); 
    // });

    // 1. Walk-in (via xc-contract on ParaB)
    ParaB::execute_with(|| {
        let sel_walk_in = encode_selector("0xc0397d90");
        let data = call_contract(&xc_contract, ALICE, sel_walk_in.encode(), 0);
        let rs: Result<(), u8> = Decode::decode(&mut &data[..]).expect("failed to decode");

        assert_eq!(rs, Ok(()));
    });

    // ParaA::execute_with(|| {
    //     let sel_counter = encode_selector("0x94fc951c");
    //     let payload = (sel_counter, 0_u128, xc_contract.clone()).encode();

    //     let data = call_contract(&xcm_handler, ALICE, payload, 0);
    //     let res: Result<u128, u8> = Decode::decode(&mut &data[..]).expect("failed to decode");
    //     assert_eq!(res, Ok(1));
    // });

    // ParaB::execute_with(|| {
    //     let sel_retrieve_counter = encode_selector("0x1002fc30");
    //     let payload = (sel_retrieve_counter, 0_u128).encode();

    //     let data = call_contract(&xc_contract, ALICE, payload, 0);
    //     let res: Result<u128, u8> = Decode::decode(&mut &data[..]).expect("failed to decode");
    //     assert_eq!(res, Ok(1));
    // })

    // Verify data stored in the state_manager
    verify_contract_state(&state_manager, 1, Some(ALICE));

    // 2. Request data for `who_am_i(id)` on ParaB
    let id: u128 = 1;
    let tid_0 = ParaB::execute_with(|| {
        let sel_who_am_i = encode_selector("0x8fb9cb05");
        let payload = (sel_who_am_i, id).encode();

        let data = call_contract(&xc_contract, ALICE, payload, 0);
        let rs: Result<u128, u8> = Decode::decode(&mut &data[..]).expect("failed to decode");
        rs.unwrap()
    });
    assert_eq!(tid_0, 0);

    // // TmpCall { Not Intended }. Manual callback from xcm-handler (ParaA)
    // ParaA::execute_with(|| {
    //     let force_who_am_i = encode_selector("0x463827b6");
    //     let xc_contract_soac = sibling_account_account_id(2, xc_contract.clone());

    //     let payload = (force_who_am_i, xc_contract_soac, tid_0, id).encode();

    //     let data = call_contract(&xcm_handler, ALICE, payload, 0);
    //     let rs: Result<Option<AccountId32>, u8> =
    //         Decode::decode(&mut &data[..]).expect("failed to decode");

    //     assert_eq!(rs, Ok(Some(ALICE)));
    // });

    // // 3. Retrieve data on ParaB
    let who_am_i = ParaB::execute_with(|| {
        let sel_retrieve_who_am_i = encode_selector("0xafc817a8");
        let payload = (sel_retrieve_who_am_i, tid_0).encode();

        let data = call_contract(&xc_contract, ALICE, payload, 0);
        let rs: Result<Option<AccountId32>, u8> =
            Decode::decode(&mut &data[..]).expect("failed to decode");
        rs
    });
    assert_eq!(who_am_i, Ok(Some(ALICE)));
}

use std::sync::Once;
static INIT: Once = Once::new();
fn init_tracing() {
    INIT.call_once(|| {
        // Add test tracing (from sp_tracing::init_for_tests()) but filtering for xcm logs only
        tracing_subscriber::fmt()
            .with_max_level(tracing::Level::TRACE)
            .with_env_filter("xcm=trace,system::events=trace,runtime::contracts=debug") // Comment out this line to see all traces
            .with_test_writer()
            .init();
    });
}
#![cfg_attr(not(feature = "std"), no_std, no_main)]

#[ink::contract]
mod demo {
    use scale::Encode;
    use xcm::v3::prelude::*;

    const PATH_TO_SEC_CHAIN: MultiLocation = MultiLocation {
        parents: 1,
        interior: X1(Parachain(2)),
    };

    #[ink(storage)]
    pub struct Demo {
        demo_count: u128,
    }

    impl Demo {
        #[ink(constructor)]
        pub fn new() -> Self {
            Self {
                demo_count: 0,
            }
        }

        // Invoked by xc_demo::call_demo
        #[ink(message)]
        pub fn increment(&mut self, callback_contract_addr: AccountId) -> Result<(), ()> {
            self.demo_count += 1;

            let selector = ink::selector_bytes!("accept_response");
            let res = utils::make_xcm_contract_call::<Self>(
                PATH_TO_SEC_CHAIN.into(),
                callback_contract_addr,
                selector.encode(),
                0,
                None,
            );
            ink::env::debug_println!("Res(demo): {:?}", res);
            
            Ok(())
        }

        // Getter
        #[ink(message)]
        pub fn get_demo_count(&self) -> u128 {
            self.demo_count
        }
    }
}

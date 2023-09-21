#![cfg_attr(not(feature = "std"), no_std, no_main)]

#[ink::contract]
mod xc_demo {
    use scale::Encode;
    use xcm::v3::prelude::*;

    const PATH_TO_HOST_CHAIN: MultiLocation = MultiLocation {
        parents: 1,
        interior: X1(Parachain(1)),
    };

    #[ink(storage)]
    pub struct XcDemo {
        demo_contract: AccountId,
        demo_count: u128,
    }

    impl XcDemo {
        #[ink(constructor)]
        pub fn new(demo_contract: AccountId) -> Self {
            Self { 
                demo_contract,
                demo_count: 0,
            }
        }

        #[ink(message)]
        pub fn call_demo(&mut self) -> Result<(), ()> {
            let selector = ink::selector_bytes!("increment");
            let payload = (selector, self.env().account_id()).encode();

            let res = utils::make_xcm_contract_call::<Self>(
                PATH_TO_HOST_CHAIN.into(),
                self.demo_contract,
                payload,
                0,
                None,
            );
            ink::env::debug_println!("Res(xc-demo): {:?}", res);
            
            Ok(())
        }

        // Getter
        #[ink(message)]
        pub fn get_demo_count(&self) -> u128 {
            self.demo_count
        }

        // Invoked by demo::increment (origin: xc_demo::call_demo)
        #[ink(message)]
        pub fn accept_response(&mut self) {
            self.demo_count += 1;
        }
    }
}

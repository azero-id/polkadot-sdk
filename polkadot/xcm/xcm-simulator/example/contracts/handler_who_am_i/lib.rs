#![cfg_attr(not(feature = "std"), no_std, no_main)]

#[ink::contract]
mod handler_who_am_i {
    use utils::make_xcm_contract_call;
    use ink::storage::Mapping;
    use scale::Encode;
    use xcm::v3::prelude::*;

    pub type SovereignAccount = AccountId;
    pub type Location = (u32, AccountId); // IMPROVE ME !!! Goal -> MultiLocation
    pub type TicketId = u128;

    #[derive(scale::Encode, scale::Decode)]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub enum ReadInterface {
        Counter(u128),
        LastVisitor(Option<AccountId>),
        WhoAmI(Option<AccountId>),
    }

    #[derive(scale::Encode, scale::Decode)]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub enum Error {
        NotAdmin,
        UnknownCaller,
        CallRuntimeFailed,
        WhoAmI(u8),
    }

    impl From<ink::env::Error> for Error {
        fn from(e: ink::env::Error) -> Self {
            match e {
                ink::env::Error::CallRuntimeFailed => Error::CallRuntimeFailed,
                _ => panic!("Unexpected error from `pallet-contracts`."),
            }
        }
    }

    #[ink::trait_definition]
    pub trait WhoAmI {
        #[ink(message, selector = 0x94fc951c)]
        fn counter(&self) -> u128;

        #[ink(message, selector = 0xef8c2bd9)]
        fn last_visitor(&self) -> Option<AccountId>;

        #[ink(message, selector = 0x8fb9cb05)]
        fn who_am_i(&self, id: u128) -> Option<AccountId>;

        #[ink(message, selector = 0xb8f6c098)]
        fn xcm_walk_in(&mut self, caller: AccountId) -> Result<(u128, AccountId), u8>;
    }

    #[ink(storage)]
    pub struct HandlerWhoAmI {
        admin: AccountId,
        who_am_i: ink::contract_ref!(WhoAmI),
        xc_contracts: Mapping<SovereignAccount, Location>,
    }

    impl HandlerWhoAmI {
        #[ink(constructor)]
        pub fn new(admin: AccountId, who_am_i: AccountId) -> Self {
            Self {
                admin,
                who_am_i: who_am_i.into(),
                xc_contracts: Mapping::default(),
            }
        }

        #[ink(message)]
        pub fn add_xc_contract(
            &mut self,
            sovereign_acc: SovereignAccount,
            location: Location,
        ) -> Result<(), Error> {
            self.ensure_admin()?;

            // check to ensure only supported Location types are added
            Self::resolve_location(location)?;
            self.xc_contracts.insert(sovereign_acc, &location);

            Ok(())
        }

        // @note does not forward the output back to origin
        #[ink(message)]
        pub fn walk_in(&mut self, origin: AccountId) -> Result<(u128, AccountId), Error> {
            self.auth_caller()?;

            let interchain_account = self.interchain_account(origin);
            self.who_am_i
                .xcm_walk_in(interchain_account)
                .map_err(Error::WhoAmI)
        }

        #[ink(message)]
        pub fn counter(&mut self, tid: TicketId) -> Result<u128, Error> {
            let (_, location) = self.auth_caller()?;

            let output = self.who_am_i.counter();
            let read_interface = ReadInterface::Counter(output);
            Self::send_response_back(location, tid, read_interface)?;

            Ok(output)
        }

        #[ink(message)]
        pub fn last_visitor(&mut self, tid: TicketId) -> Result<Option<AccountId>, Error> {
            let (_, location) = self.auth_caller()?;

            let output = self.who_am_i.last_visitor();
            let read_interface = ReadInterface::LastVisitor(output);
            Self::send_response_back(location, tid, read_interface)?;

            Ok(output)
        }

        #[ink(message)]
        pub fn who_am_i(&mut self, tid: TicketId, id: u128) -> Result<Option<AccountId>, Error> {
            let (_, location) = self.auth_caller()?;

            let output = self.who_am_i.who_am_i(id);
            let read_interface = ReadInterface::WhoAmI(output);
            Self::send_response_back(location, tid, read_interface)?;

            Ok(output)
        }

        // For debugging purpose
        #[ink(message)]
        pub fn force_who_am_i(
            &mut self,
            caller: AccountId,
            tid: TicketId,
            id: u128,
        ) -> Result<Option<AccountId>, Error> {
            let Some(location) = self.xc_contracts.get(caller) else {
                Err(Error::UnknownCaller)?
            };

            let output = self.who_am_i.who_am_i(id);
            let read_interface = ReadInterface::WhoAmI(output);
            Self::send_response_back(location, tid, read_interface)?;

            Ok(output)
        }

        fn send_response_back(
            location: Location,
            tid: TicketId,
            read_interface: ReadInterface,
        ) -> Result<(), Error> {
            let (path_to_chain, contract_address) = Self::resolve_location(location)?;

            let selector = ink::selector_bytes!("accept_response");
            let read_interface: ink::prelude::vec::Vec<u8> = read_interface.encode();

            make_xcm_contract_call::<Self>(
                path_to_chain.into(),
                contract_address,
                (selector, tid, read_interface).encode(),
                0,
                None,
            )
            .map_err(Into::into)
        }

        fn resolve_location(location: Location) -> Result<(MultiLocation, AccountId), Error> {
            let (para_id, contract_addr) = location;
            let path_to_chain: MultiLocation = (Parent, Parachain(para_id)).into();

            Ok((path_to_chain, contract_addr))
        }

        // Gives us the control to have our own form of interchain account
        // Ideally should comply w/ the chains' Sovereign Account for consistency
        fn interchain_account(&self, origin: AccountId) -> AccountId {
            origin // Alias Mode ON
        }

        fn auth_caller(&self) -> Result<(AccountId, Location), Error> {
            let caller = self.env().caller();

            match self.xc_contracts.get(caller) {
                Some(loc) => Ok((caller, loc)),
                None => Err(Error::UnknownCaller),
            }
        }

        fn ensure_admin(&self) -> Result<(), Error> {
            if self.env().caller() != self.admin {
                Err(Error::NotAdmin)?;
            }
            Ok(())
        }
    }
}

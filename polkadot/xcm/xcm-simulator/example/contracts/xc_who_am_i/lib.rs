#![cfg_attr(not(feature = "std"), no_std, no_main)]

#[ink::contract]
mod xc_who_am_i {
    use utils::make_xcm_contract_call;
    use ink::prelude::vec::Vec;
    use ink::storage::Mapping;
    use scale::{Decode, Encode};
    use xcm::v3::prelude::*;

    const PATH_TO_HOST_CHAIN: MultiLocation = MultiLocation {
        parents: 1,
        interior: X1(Parachain(1)),
    };

    #[derive(scale::Encode, scale::Decode)]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub enum ReadInterface {
        Counter(u128),
        LastVisitor(Option<AccountId>),
        WhoAmI(Option<AccountId>),
    }

    pub type ReadInterfaceEncoded = Vec<u8>; // FIXME: Could not store `ReadInterface` in storage
    pub type TicketId = u128;

    #[derive(Debug, PartialEq, Eq, scale::Encode, scale::Decode)]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub enum Error {
        NotHandler,
        CallRuntimeFailed,
        InvalidTicketId,
        AwaitingResponse,
        FailedToDecodeResponse,
        TicketIdMismatch,
        DuplicateResponse,
    }

    impl From<ink::env::Error> for Error {
        fn from(e: ink::env::Error) -> Self {
            match e {
                ink::env::Error::CallRuntimeFailed => Error::CallRuntimeFailed,
                _ => panic!("Unexpected error from `pallet-contracts`."),
            }
        }
    }

    #[ink(event)]
    pub struct ResponseReceived {
        #[ink(topic)]
        ticket_id: TicketId,
    }

    #[ink(storage)]
    pub struct XcWhoAmI {
        contract_addr: AccountId,
        sovereign_contract_addr: AccountId, // Try computing it on-chain
        ticket_counter: u128,
        ticket_to_response: Mapping<TicketId, ReadInterfaceEncoded>,
    }

    impl XcWhoAmI {
        #[ink(constructor)]
        pub fn new(contract_addr: AccountId, sovereign_contract_addr: AccountId) -> Self {
            Self {
                contract_addr,
                sovereign_contract_addr,
                ticket_counter: 0,
                ticket_to_response: Mapping::default(),
            }
        }

        #[ink(message)]
        pub fn total_tickets_issued(&self) -> u128 {
            self.ticket_counter
        }

        #[ink(message)]
        pub fn walk_in(&mut self) -> Result<(), Error> {
            let selector = ink::selector_bytes!("walk_in");
            let caller = self.env().caller();

            self.call_handler((selector, caller).encode())
        }

        #[ink(message)]
        pub fn counter(&mut self) -> Result<TicketId, Error> {
            let tid = self.ticket_counter;

            let selector = ink::selector_bytes!("counter");
            self.call_handler((selector, tid).encode())?;

            self.ticket_counter += 1;
            Ok(tid)
        }

        #[ink(message)]
        pub fn last_visitor(&mut self) -> Result<TicketId, Error> {
            let tid = self.ticket_counter;

            let selector = ink::selector_bytes!("last_visitor");
            self.call_handler((selector, tid).encode())?;

            self.ticket_counter += 1;
            Ok(tid)
        }

        #[ink(message)]
        pub fn who_am_i(&mut self, id: u128) -> Result<TicketId, Error> {
            let tid = self.ticket_counter;

            let selector = ink::selector_bytes!("who_am_i");
            self.call_handler((selector, tid, id).encode())?;

            self.ticket_counter += 1;
            Ok(tid)
        }

        #[ink(message)]
        pub fn retrieve_counter(&self, tid: TicketId) -> Result<u128, Error> {
            match self.read_response(tid)? {
                ReadInterface::Counter(v) => Ok(v),
                _ => Err(Error::TicketIdMismatch),
            }
        }

        #[ink(message)]
        pub fn retrieve_last_visitor(&self, tid: TicketId) -> Result<Option<AccountId>, Error> {
            match self.read_response(tid)? {
                ReadInterface::LastVisitor(v) => Ok(v),
                _ => Err(Error::TicketIdMismatch),
            }
        }

        #[ink(message)]
        pub fn retrieve_who_am_i(&self, tid: TicketId) -> Result<Option<AccountId>, Error> {
            match self.read_response(tid)? {
                ReadInterface::WhoAmI(v) => Ok(v),
                _ => Err(Error::TicketIdMismatch),
            }
        }

        #[ink(message)]
        pub fn accept_response(&mut self, tid: TicketId, response: Vec<u8>) -> Result<(), Error> {
            self.ensure_handler()?;

            if tid >= self.ticket_counter {
                Err(Error::InvalidTicketId)?
            } else if self.ticket_to_response.contains(tid) {
                Err(Error::DuplicateResponse)?
            }

            self.ticket_to_response.insert(tid, &response);

            // Emit event to announce response availability
            self.env().emit_event(ResponseReceived { ticket_id: tid });

            Ok(())
        }

        #[ink(message)]
        pub fn read_response(&self, tid: TicketId) -> Result<ReadInterface, Error> {
            if tid >= self.ticket_counter {
                Err(Error::InvalidTicketId)?
            }

            let Some(response) = self.ticket_to_response.get(tid) else {
                Err(Error::AwaitingResponse)?
            };

            ReadInterface::decode(&mut &response[..]).map_err(|_| Error::FailedToDecodeResponse)
        }

        #[ink(message)]
        pub fn read_raw_response(&self, tid: TicketId) -> Option<Vec<u8>> {
            self.ticket_to_response.get(tid)
        }

        fn call_handler(&mut self, payload: Vec<u8>) -> Result<(), Error> {
            make_xcm_contract_call::<Self>(
                PATH_TO_HOST_CHAIN.into(),
                self.contract_addr,
                payload,
                0,
                None,
            )
            .map_err(Into::into)
        }

        fn ensure_handler(&self) -> Result<(), Error> {
            if self.env().caller() != self.sovereign_contract_addr {
                Err(Error::NotHandler)?
            }
            Ok(())
        }
    }
}

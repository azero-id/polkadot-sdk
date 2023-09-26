#![cfg_attr(not(feature = "std"), no_std, no_main)]

#[ink::contract]
mod who_am_i {
    use ink::storage::Mapping;

    #[ink(event)]
    pub struct UserWalkedIn {
        id: u128,
        user: AccountId,
    }

    #[derive(scale::Encode, scale::Decode)]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub enum Error {
        NotAdmin,
        NotHandler,
    }

    #[ink(storage)]
    pub struct WhoAmI {
        admin: AccountId,
        handler: AccountId,
        counter: u128,
        visitors: Mapping<u128, AccountId>,
    }

    impl WhoAmI {
        #[ink(constructor)]
        pub fn new(admin: AccountId, handler: AccountId) -> Self {
            Self {
                admin,
                handler,
                counter: 0,
                visitors: Mapping::default(),
            }
        }

        #[ink(message)]
        pub fn set_handler(&mut self, new_handler: AccountId) -> Result<(), Error> {
            self.ensure_admin()?;
            self.handler = new_handler;
            Ok(())
        }

        #[ink(message)]
        pub fn counter(&self) -> u128 {
            self.counter
        }

        #[ink(message)]
        pub fn last_visitor(&self) -> Option<AccountId> {
            self.visitors.get(self.counter)
        }

        #[ink(message)]
        pub fn who_am_i(&self, id: u128) -> Option<AccountId> {
            self.visitors.get(id)
        }

        #[ink(message)]
        pub fn walk_in(&mut self) -> (u128, AccountId) {
            let caller = self.env().caller();
            self.do_walk_in(&caller)
        }

        #[ink(message)]
        pub fn xcm_walk_in(&mut self, caller: AccountId) -> Result<(u128, AccountId), Error> {
            self.ensure_handler()?;
            Ok(self.do_walk_in(&caller))
        }

        fn do_walk_in(&mut self, caller: &AccountId) -> (u128, AccountId) {
            self.counter += 1;
            self.visitors.insert(self.counter, caller);

            self.env().emit_event(UserWalkedIn {
                id: self.counter,
                user: *caller,
            });

            (self.counter, *caller)
        }

        fn ensure_admin(&self) -> Result<(), Error> {
            if self.env().caller() != self.admin {
                Err(Error::NotAdmin)?;
            }
            Ok(())
        }

        fn ensure_handler(&self) -> Result<(), Error> {
            if self.env().caller() != self.handler {
                Err(Error::NotHandler)?;
            }
            Ok(())
        }
    }
}

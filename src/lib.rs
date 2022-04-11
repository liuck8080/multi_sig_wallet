#![no_std]

pub mod wallet;
use wallet::{uint, MultiSigWallet};
use gstd::{msg, prelude::*, ActorId};

#[derive(Debug, Decode, Encode, TypeInfo)]
pub enum Action {
    AddOwner(ActorId),
    RemoveOwner(ActorId),
    ReplaceOwner{from: ActorId, to: ActorId},
    ChangeRequirement{required:uint},
    SubmitTransaction{destination:ActorId, value:uint, data:Vec<u8>},
    ConfirmTransaction{transaction_id:uint},
    RevokeConfirmation{transaction_id:uint},
    ExecuteTransaction{transaction_id:uint},
    IsConfirmed{transaction_id:uint},
    GetConfirmationCount{transaction_id:uint},
    GetTransactionCount{pending:bool, executed:bool},
    GetOwners,
    GetConfirmations{transaction_id:uint},
    GetTransactionIds{from:uint, to:uint, pending:bool, executed:bool}
}


#[derive(Debug, Encode, Decode, TypeInfo)]
pub enum Event {
    Invalid,
    Confirmation {sender: ActorId, transaction_id: uint, executed: bool},
    Revocation{sender:ActorId, transaction_id:uint},
    Submission{transaction_id:uint},
    Execution{transaction_id:uint},
    ExecutionFailure{transaction_id:uint},
    IsConfirmed(bool),
    GetConfirmationCount(uint),
    GetTransactionCount(uint),
    Deposit{sender: ActorId, value:uint},
    OwnerAddition{owner:ActorId},
    OwnerRemoval{owner:ActorId},
    OwnerReplace{from:ActorId, to:ActorId},
    RequirementChange{from: uint, to: uint},
    GetConfirmations(Vec<ActorId>),
    GetTransactionIds(Vec<uint>),
    GetOwners(Vec<ActorId>),
}

#[derive(Debug, Decode, Encode, TypeInfo)]
pub struct InitConfig {
    pub owners: Vec<ActorId>,
    pub required: uint,
}

static mut WALLET: Option<MultiSigWallet> = None;

#[no_mangle]
pub unsafe extern "C" fn init() {
    let config: InitConfig = msg::load().expect("Unable to decode InitConfig");
    let ft = wallet::MultiSigWallet::new(&config.owners, config.required);
    WALLET = Some(ft);
}

gstd::metadata! {
    title: "FungibleToken",
    init:
        input: InitConfig,
    handle:
        input: Action,
        output: Event,
}

#[gstd::async_main]
async unsafe fn main() {
    let action: Action = msg::load().expect("Could not load Action");
    let wallet: &mut MultiSigWallet = unsafe {WALLET.get_or_insert(MultiSigWallet::default())};
    match action {
        Action::AddOwner(owner) => {
            wallet.add_owner(&owner);

            msg::reply(Event::OwnerAddition{owner}, 0);
        }
        Action::RemoveOwner(owner) => {
            wallet.remove_owner(&owner);

            msg::reply(Event::OwnerRemoval{owner}, 0);
        }
        Action::ReplaceOwner{from, to} => {
            wallet.replace_owner(&from, &to);

            msg::reply(Event::OwnerReplace{from,to}, 0);
        }
        Action::ChangeRequirement{required} => {
            wallet.change_requirement(required);
            let from = wallet.get_required();

            msg::reply(Event::RequirementChange{from, to:required}, 0);
        }
        Action::SubmitTransaction{destination, value, data} => {
            let id = wallet.submit_transaction(&msg::source(), &destination, &value, &data).await;
            msg::reply(Event::Submission{transaction_id:id}, 0);
        }
        Action::ConfirmTransaction{transaction_id} => {
            let i = wallet.confirm_transaction(&msg::source(), &transaction_id).await;
            msg::reply(Event::Confirmation{sender:msg::source().clone(), transaction_id: transaction_id.clone(), executed: i == 1}, 0);
        }
        Action::RevokeConfirmation{transaction_id} => {
            wallet.revoke_confirmation(&msg::source(), &transaction_id);
            msg::reply(Event::Revocation{sender:msg::source(), transaction_id:transaction_id}, 0);
        }
        Action::ExecuteTransaction{transaction_id} => {
            let i = wallet.execute_transaction(&msg::source(), &transaction_id).await;
            match i {
                 0 => {},
                 1 => {msg::reply(Event::Execution{transaction_id: transaction_id}, 0);},
                 2 => {msg::reply(Event::ExecutionFailure{transaction_id: transaction_id}, 0);},
                 _ => {},
            }
        }
        Action::IsConfirmed{transaction_id} => {
            let c = wallet.is_confirmed(&transaction_id);
            msg::reply(Event::IsConfirmed(c), 0);
        }
        Action::GetConfirmationCount{transaction_id} => {
            let cc = wallet.get_confirmation_count(&transaction_id);

            msg::reply(Event::GetConfirmationCount(cc), 0);
        }
        Action::GetTransactionCount{pending, executed} => {
            let tc =  wallet.get_transaction_count(pending, executed);
            msg::reply(Event::GetTransactionCount(tc), 0);
        }
        Action::GetOwners => {
            let owners = wallet.get_owners();
            msg::reply(Event::GetOwners(owners), 0);
        }
        Action::GetConfirmations{transaction_id} => {
            let confirmations = wallet.get_confirmations(&transaction_id);

            msg::reply(Event::GetConfirmations(confirmations), 0);
        }
        Action::GetTransactionIds{from, to, pending, executed} => {
            let ids = wallet.get_transaction_ids(&from, &to, pending, executed);

            msg::reply(Event::GetTransactionIds(ids), 0);
        }
    }
}


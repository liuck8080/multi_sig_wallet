use gtest::{Log, Program, System};
use gstd::{prelude::*, ActorId};
// use gear_core::ids::ProgramId;
use multi_sig_wallet::*;
use multi_sig_wallet::wallet::*;

const REQUIRED: uint = 02;
const FROM_ID:u64 = 100001;

fn actor2arr(id:&ActorId) ->[u8;32] {
    let mut ans = [0u8; 32];
    let base = id.as_ref();
    assert!(base.len() == 32);
    ans[..].copy_from_slice(base);
    ans 
}
// fn programId2ActorId(id:&ProgramId) ->ActorId {
//     ActorId::from_slice(id.as_ref()).unwrap()
// }
fn create_owner(x:u8)->ActorId {
    let mut y = [0u8;32];
    for i in 0..32 {
        y[i] = x;
    }
    ActorId::new(y)
}

fn init(sys:&System) {
    sys.spend_blocks(150);
    sys.init_logger();
}

fn send_init(owners:&Vec<ActorId>, program:&Program) {
    let init = InitConfig {
        owners: owners.to_owned(),
        required: REQUIRED,
    };
    let res = program.send_bytes(FROM_ID, init.encode());
    assert!(res.log().is_empty());
    assert!(!res.main_failed());
    assert!(!res.others_failed());
    verify_owners(program, owners);
}

fn verify_owners(program:&Program, owners:&Vec<ActorId>) {
    let res = program.send(FROM_ID, Action::GetOwners);
    let event = Event::GetOwners(owners.clone());
    assert!(res.contains(&Log::builder().payload(event)));
}

fn add_owner(owners:&mut Vec<ActorId>, program:&Program) {
    let owner5 = create_owner(5u8);

    // befor confirm
    let action = Action::AddOwner(owner5.clone());
    let transaction_id = submit_transaction(program, &owners[0], &action.encode());
    assert!(1 == get_confirmation_count(program, &transaction_id));
    assert!(!is_confirmed(program, &transaction_id));
    let old_confirmations = get_confirmations(program, &transaction_id);
    assert!(1== old_confirmations.len());
    assert!(old_confirmations[0]==owners[0]);
    verify_owners(program, owners);
    assert!(0 == get_transaction_count(program, false, true));
    assert!(1 == get_transaction_count(program, true, false));
    assert!(1 == get_transaction_count(program, true, true));
    // confirm
    let e = confirm_transaction(program, &owners[1], &transaction_id);
    if let Event::Confirmation{sender, transaction_id:id, executed} =  e{
        assert!(sender == owners[1]);
        assert!(transaction_id==id);
        assert!(executed);
    }

    // after confirm
    assert!(2 == get_confirmation_count(program, &transaction_id));
    assert!(is_confirmed(program, &transaction_id));
    let new_confirmations = get_confirmations(program, &transaction_id);
    assert!(2== new_confirmations.len());
    assert!(new_confirmations[0]==owners[0]);
    assert!(new_confirmations[1]==owners[1]);

    owners.push(owner5.clone());
    verify_owners(program, owners);

    assert!(1 == get_transaction_count(program, false, true));
    assert!(0 == get_transaction_count(program, true, false));
    assert!(1 == get_transaction_count(program, true, true));
}

fn remove_owner(owners:&mut Vec<ActorId>, program:&Program) {
    let owner5 = owners.last().unwrap().clone();

    // befor confirm
    let action = Action::RemoveOwner(owner5.clone());
    let transaction_id = submit_transaction(program, &owners[0], &action.encode());
    assert!(1 == get_confirmation_count(program, &transaction_id));
    assert!(!is_confirmed(program, &transaction_id));
    let old_confirmations = get_confirmations(program, &transaction_id);
    assert!(1== old_confirmations.len());
    assert!(old_confirmations[0]==owners[0]);
    verify_owners(program, owners);
    assert!(1 == get_transaction_count(program, false, true));
    assert!(1 == get_transaction_count(program, true, false));
    assert!(2 == get_transaction_count(program, true, true));
    // confirm
    let e = confirm_transaction(program, &owners[1], &transaction_id);
    if let Event::Confirmation{sender, transaction_id:id, executed} =  e{
        assert!(sender == owners[1]);
        assert!(transaction_id==id);
        assert!(executed);
    }

    // after confirm
    assert!(2 == get_confirmation_count(program, &transaction_id));
    assert!(is_confirmed(program, &transaction_id));
    let new_confirmations = get_confirmations(program, &transaction_id);
    assert!(2== new_confirmations.len());
    assert!(new_confirmations[0]==owners[0]);
    assert!(new_confirmations[1]==owners[1]);

    // owners.push(owner5.clone());
    owners.pop();
    verify_owners(program, owners);

    assert!(2 == get_transaction_count(program, false, true));
    assert!(0 == get_transaction_count(program, true, false));
    assert!(2 == get_transaction_count(program, true, true));
}

fn change_requirement(owners:&Vec<ActorId>, program:&Program) {
    // befor confirm
    let action = Action::ChangeRequirement{required:REQUIRED + 1};
    let transaction_id = submit_transaction(program, &owners[0], &action.encode());
    assert!(1 == get_confirmation_count(program, &transaction_id));
    assert!(!is_confirmed(program, &transaction_id));
    let old_confirmations = get_confirmations(program, &transaction_id);
    assert!(1== old_confirmations.len());
    assert!(old_confirmations[0]==owners[0]);
    verify_owners(program, owners);
    assert!(2 == get_transaction_count(program, false, true));
    assert!(1 == get_transaction_count(program, true, false));
    assert!(3 == get_transaction_count(program, true, true));
    // confirm
    let e = confirm_transaction(program, &owners[1], &transaction_id);
    if let Event::Confirmation{sender, transaction_id:id, executed} =  e{
        assert!(sender == owners[1]);
        assert!(transaction_id==id);
        assert!(executed);
    }

    // after confirm
    assert!(2 == get_confirmation_count(program, &transaction_id));
    assert!(!is_confirmed(program, &transaction_id)); // since required changed, should not confirmed
    let new_confirmations = get_confirmations(program, &transaction_id);
    assert!(2== new_confirmations.len());
    assert!(new_confirmations[0]==owners[0]);
    assert!(new_confirmations[1]==owners[1]);


    assert!(3 == get_transaction_count(program, false, true));
    assert!(0 == get_transaction_count(program, true, false));
    assert!(3 == get_transaction_count(program, true, true));
}

fn replace_owner(owners:&mut Vec<ActorId>, program:&Program) {
    let owner6 = create_owner(6u8);

    // befor confirm
    let action = Action::ReplaceOwner{from:owners[3].clone(), to:owner6};
    let transaction_id = submit_transaction(program, &owners[0], &action.encode());
    assert!(1 == get_confirmation_count(program, &transaction_id));
    assert!(!is_confirmed(program, &transaction_id));
    let old_confirmations = get_confirmations(program, &transaction_id);
    assert!(1== old_confirmations.len());
    assert!(old_confirmations[0]==owners[0]);
    verify_owners(program, owners);
    assert!(3 == get_transaction_count(program, false, true));
    assert!(1 == get_transaction_count(program, true, false));
    assert!(4 == get_transaction_count(program, true, true));
    // confirm
    let e = confirm_transaction(program, &owners[1], &transaction_id);
    if let Event::Confirmation{sender, transaction_id:id, executed} =  e{
        assert!(sender == owners[1]);
        assert!(transaction_id==id);
        assert!(!executed);
    }

    // not enough confirmations
    assert!(2 == get_confirmation_count(program, &transaction_id));
    assert!(!is_confirmed(program, &transaction_id));
    let new_confirmations = get_confirmations(program, &transaction_id);
    assert!(2== new_confirmations.len());
    assert!(new_confirmations[0]==owners[0]);
    assert!(new_confirmations[1]==owners[1]);
    // 3 confirmations
    let e = confirm_transaction(program, &owners[2], &transaction_id);
    if let Event::Confirmation{sender, transaction_id:id, executed} =  e{
        assert!(sender == owners[2]);
        assert!(transaction_id==id);
        assert!(executed);
    }
    assert!(3 == get_confirmation_count(program, &transaction_id));
    assert!(is_confirmed(program, &transaction_id));
    let new_confirmations = get_confirmations(program, &transaction_id);
    assert!(3== new_confirmations.len());
    assert!(new_confirmations[0]==owners[0]);
    assert!(new_confirmations[1]==owners[1]);
    assert!(new_confirmations[2]==owners[2]);


    // owners.push(owner5.clone());
    owners[3] = owner6;
    verify_owners(program, owners);

    assert!(4 == get_transaction_count(program, false, true));
    assert!(0 == get_transaction_count(program, true, false));
    assert!(4== get_transaction_count(program, true, true));
}

fn submit_transaction(program:&Program,sender:&ActorId, data:&[u8])-> uint {
    let destination = {
        ActorId::from_slice(program.id().as_ref()).unwrap()
    };
    let res = program.send(actor2arr(sender), Action::SubmitTransaction{destination:destination, value:0, data:data.to_vec() });
    assert!(!res.log().is_empty());
    assert!(!res.main_failed());
    assert!(!res.others_failed());
    
    for log in res.log() {
        if let Ok(Event::Submission{transaction_id}) = Event::decode(&mut log.payload().as_ref()){
            return transaction_id;
        }
    }
    0
}

fn get_confirmation_count(program:&Program, transaction_id:&uint)->uint {
    let action = Action::GetConfirmationCount{transaction_id:*transaction_id};
    let res = program.send(FROM_ID, action);
    assert!(!res.log().is_empty());
    assert!(!res.main_failed());
    assert!(!res.others_failed());
    for log in res.log() {
        if let Ok(Event::GetConfirmationCount(cnt)) = Event::decode(&mut log.payload().as_ref()){
            return cnt;
        }
    }
    assert!(false, "should not reach here");
    0
}

fn is_confirmed(program:&Program, transaction_id:&uint)->bool {
    let action = Action::IsConfirmed{transaction_id:*transaction_id};
    let res = program.send(FROM_ID, action);
    assert!(!res.log().is_empty());
    assert!(!res.main_failed());
    assert!(!res.others_failed());
    for log in res.log() {
        if let Ok(Event::IsConfirmed(c)) = Event::decode(&mut log.payload().as_ref()){
            return c;
        }
    }

    assert!(false, "should not reach here");
    false
}

fn get_confirmations(program:&Program, transaction_id:&uint)->Vec<ActorId> {
    let action = Action::GetConfirmations{transaction_id:*transaction_id};
    let res = program.send(FROM_ID, action);
    assert!(!res.log().is_empty());
    assert!(!res.main_failed());
    assert!(!res.others_failed());
    for log in res.log() {
        if let Ok(Event::GetConfirmations(c)) = Event::decode(&mut log.payload().as_ref()){
            return c;
        }
    }

    assert!(false, "should not reach here");
    vec![]
}

fn confirm_transaction(program:&Program, owner:&ActorId, transaction_id:&uint)->Event {
    let action = Action::ConfirmTransaction{transaction_id:*transaction_id};
    let res = program.send(actor2arr(&owner), action);
    assert!(!res.log().is_empty());
    assert!(!res.main_failed());
    assert!(!res.others_failed());
    for log in res.log() {
        if let Ok( e) = Event::decode(&mut log.payload().as_ref()){
            return e;
        }
    }

    assert!(false, "should not reach here");
    Event::Invalid
}

fn get_transaction_count(program:&Program, pending:bool, executed:bool) ->uint {
    let action = Action::GetTransactionCount{pending, executed};
    let res = program.send(FROM_ID, action);
    assert!(!res.log().is_empty());
    assert!(!res.main_failed());
    assert!(!res.others_failed());
    for log in res.log() {
        if let Ok( Event::GetTransactionCount(tc)) = Event::decode(&mut log.payload().as_ref()){
            return tc;
        }
    }

    assert!(false, "should not reach here");
    0
}

#[test]
fn basics() {
    let sys = System::new();
    init(&sys);
    let program = Program::from_file(
        &sys,
        "./target/wasm32-unknown-unknown/debug/multi_sig_wallet.wasm",
    );
    let mut owners = (1..5).map(|x|create_owner(x)).collect::<Vec<_>>();
    // init
    send_init(&owners, &program);
    // add owner
    add_owner(&mut owners, &program);
    remove_owner(&mut owners, &program);
    change_requirement(&owners, &program);
    replace_owner(&mut owners, &program);
}

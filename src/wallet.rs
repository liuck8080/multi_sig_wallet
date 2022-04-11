
use gstd::{exec, msg, prelude::*, ActorId};

#[allow(non_camel_case_types)]
pub type uint = u128;

const ZERO_ID: ActorId = ActorId::new([0u8; 32]);

    /*
     *  Constants
     */
pub static MAX_OWNER_COUNT:uint = 50;
#[derive(Default)]
struct Transaction {
    destination:ActorId ,
    value:uint ,
    data:Vec<u8>,
    executed:bool,
}
#[derive(Default)]
pub struct MultiSigWallet {
    transactions:  BTreeMap<uint, Transaction>,
    confirmations: BTreeMap<uint, BTreeMap<ActorId, bool>>,
    is_owner :BTreeMap<ActorId, bool> ,
    owners: Vec<ActorId>,
    required: uint,
    transaction_count: uint,
}

impl MultiSigWallet {
    #[inline]
    pub(crate) fn get_required(&self) -> uint {self.required}

    /*
     *  Modifiers
     */
    #[inline]
    fn only_wallet(&self) {
        let source = msg::source();
        let program_id = exec::program_id();

        assert!(source == program_id, "source({:?}) != program_id({:?})", source, program_id);
    }

    #[inline]
    fn owner_does_not_exist(&self, owner:&ActorId) {
        assert!(!self.is_owner.get(owner).unwrap_or(&false));
    }
    #[inline]
    fn owner_exists(&self, owner:&ActorId) {
        assert!(self.is_owner.get(owner).unwrap_or(&false));
    }

    #[inline]
    fn confirmed(&self, transaction_id:&uint, owner:&ActorId) {
        // assert!(self.confirmations[transaction_id][owner]);
        if let Some(user_confirmed) = self.confirmations.get(transaction_id) {
            if let Some(c) = user_confirmed.get(owner) {
                assert!(c);
                return;
            }
        }
        panic!("not confirmed");
    }
    #[inline]
    fn not_confirmed(&self, transaction_id:&uint, owner:&ActorId) {
        //assert!(!self.confirmations[transaction_id][owner]);
        if let Some(user_confirmed) = self.confirmations.get(transaction_id) {
            if let Some(confirmed) = user_confirmed.get(owner) {
                assert!(!confirmed);
            }
        }
    }
    #[inline]
    fn not_executed(&self, transaction_id:&uint) {
        // assert!(!self.transactions[transaction_id].executed);
        if let Some(transaction) = self.transactions.get(transaction_id) {
            assert!(!transaction.executed);
        }
    }
    #[inline]
    fn not_null(_address:&ActorId) {
        assert!(_address != &ZERO_ID);
    }
    #[inline]
    fn valid_requirement(owner_count:uint, _required:uint) {
        assert!(owner_count <= MAX_OWNER_COUNT
            && _required <= owner_count
            && _required != 0
            && owner_count != 0);
    }
    // /// @dev Fallback function allows to deposit ether.
    // function()
    // payable
    // {
    //     if (msg.value > 0)
    //         Deposit(msg.sender, msg.value);
    // }
        /*
     * Public functions
     */
    /// @dev Contract constructor sets initial owners and required number of confirmations.
    /// @param _owners List of initial owners.
    /// @param _required Number of required confirmations.
    pub fn new(_owners:&[ActorId], _required:uint)-> Self
    {
        let len = _owners.len() as uint;
        Self::valid_requirement(len, _required);
        let mut ret = Self {
            transactions: BTreeMap::new(),
            confirmations: BTreeMap::new(),
            is_owner: BTreeMap::new(),
            owners: vec![],
            required: _required ,
            transaction_count: 0,
        };
        for owner in _owners {
            assert!(ZERO_ID != *owner && !ret.is_owner.get(owner).unwrap_or(&false));
            ret.is_owner.insert(owner.to_owned(), true);
        }
        ret.owners = _owners.to_vec();
        ret.required = _required;
        ret
    }

    /// @dev Allows to add a new owner. Transaction has to be sent by wallet.
    /// @param owner Address of new owner.
    pub fn add_owner(&mut self, owner:&ActorId)
    {
        self.only_wallet();
        self.owner_does_not_exist(owner);
        Self::not_null(owner);
        Self::valid_requirement((self.owners.len() + 1) as uint, self.required);
        self.is_owner.insert(owner.clone(), true);
        self.owners.push(owner.clone());
        // OwnerAddition(owner);
    }


    /// @dev Allows to remove an owner. Transaction has to be sent by wallet.
    /// @param owner Address of owner.
    pub fn remove_owner(&mut self, owner:&ActorId)
    {        
        self.only_wallet();
        self.owner_exists(owner);
        self.is_owner.entry(owner.clone()).and_modify(|e|*e = false).or_insert(false);
        let idx = self.owners.iter().position(|x|x == owner).unwrap();
        self.owners.swap_remove(idx);
        if self.required > self.owners.len().try_into().unwrap() {
            self.change_requirement(self.owners.len().try_into().unwrap());
        }
    }

    /// @dev Allows to replace an owner with a new owner. Transaction has to be sent by wallet.
    /// @param owner Address of owner to be replaced.
    /// @param newOwner Address of new owner.
    pub fn replace_owner(&mut self, owner:&ActorId, new_owner:&ActorId)        
    {
        self.only_wallet();
        self.owner_exists(owner);
        self.owner_does_not_exist(new_owner);
        let idx = self.owners.iter().position(|x|x == owner).unwrap();
        self.owners[idx] = new_owner.clone();
        self.is_owner.insert(owner.clone(), false);
        self.is_owner.insert(new_owner.clone(), true);
    }

    /// @dev Allows to change the number of required confirmations. Transaction has to be sent by wallet.
    /// @param _required Number of required confirmations.
    pub fn change_requirement(&mut self, _required:uint)
    {
        self.only_wallet();
        Self::valid_requirement(self.owners.len().try_into().unwrap(), _required);
        self.required = _required;
        // RequirementChange(_required);
    }

    /// @dev Allows an owner to submit and confirm a transaction.
    /// @param destination Transaction target address.
    /// @param value Transaction ether value.
    /// @param data Transaction data payload.
    /// @return Returns transaction ID.
    pub async fn submit_transaction(&mut self, sender:&ActorId, destination:&ActorId, value:&uint, data:&[u8])->uint
    {
        let transaction_id = self.add_transaction(destination, value, data);
        self.confirm_transaction(sender, &transaction_id).await;
        transaction_id
    }

    /// @dev Allows an owner to confirm a transaction.
    /// @param transactionId Transaction ID.
    pub async fn confirm_transaction(&mut self, sender:&ActorId, transaction_id:&uint)->i32
    {
        self.owner_exists(sender);
        // self.transaction_exists(transaction_id);
        assert!(self.transactions.get(transaction_id).unwrap().destination != ZERO_ID);

        self.not_confirmed(transaction_id, sender);
        self.confirmations.entry(transaction_id.clone()).or_insert_with(||BTreeMap::new()).entry(sender.clone()).and_modify(|e| *e = true).or_insert(true);
        // Confirmation(msg.sender, transaction_id);
        self.execute_transaction(sender, transaction_id).await
    }

    /// @dev Allows an owner to revoke a confirmation for a transaction.
    /// @param transactionId Transaction ID.
    pub fn revoke_confirmation(&mut self, sender:&ActorId, transaction_id:&uint)
    {
        self.owner_exists(sender);
        self.confirmed(transaction_id, sender);
        self.not_executed(transaction_id);
        // self.confirmations.entry[transactionId][msg.sender] = false;
        self.confirmations.entry(*transaction_id).or_insert_with(||BTreeMap::new()).entry(*sender).and_modify(|e|*e = false).or_insert(false);
        // Revocation(msg.sender, transaction_id);
    }

    /// @dev Allows anyone to execute a confirmed transaction.
    /// @param transactionId Transaction ID.
    pub async fn execute_transaction(&mut self, sender:&ActorId, transaction_id:&uint)->i32
    {
        self.owner_exists(sender);
        self.confirmed(transaction_id, sender);
        self.not_executed(transaction_id);
        if self.is_confirmed(transaction_id) {
            let mut txn = self.transactions.get_mut(transaction_id).unwrap();
            if Self::external_call(&txn.destination, &txn.value, &txn.data).await {
                txn.executed = true;
                // Execution(transactionId);
                return 1;
            }
            else {
                // ExecutionFailure(transactionId);
                txn.executed = false;
                return 2;
            }
        }
        0
    }

    // call has been separated into its own fn in order to take advantage
    // of the Solidity's code generator to produce a loop that copies tx.data into memory.
    async fn external_call(destination:&ActorId, value:&uint, data:&[u8]) -> bool {
        match msg::send_bytes_and_wait_for_reply(destination.to_owned(), data, value.to_owned()).await {
            Ok(_bytes) => {
                // msg::reply_bytes(bytes, 0);
                true
            },
            Err(_e)   => false,
        }
    }

    /// @dev Returns the confirmation status of a transaction.
    /// @param transactionId Transaction ID.
    /// @return Confirmation status.
    pub fn is_confirmed(&self, transaction_id:&uint)->bool
    {
        let mut count = 0;
        let cfm_dict = self.confirmations.get(transaction_id).unwrap();
        for confirmed in cfm_dict.values() {
            if *confirmed {
                count += 1;
            }
            if count == self.required {
                return true;
            }
        }

        false
    }

    /*
     * Internal fns
     */
    /// @dev Adds a new transaction to the transaction mapping, if transaction does not exist yet.
    /// @param destination Transaction target address.
    /// @param value Transaction ether value.
    /// @param data Transaction data payload.
    /// @return Returns transaction ID.
    fn add_transaction(&mut self, destination:&ActorId, value:&uint, data:&[u8])->uint
    {
        Self::not_null(destination);
        let transaction_id = self.transaction_count;
        self.transactions.insert(transaction_id, Transaction{
            destination:destination.clone(),
            value: *value,
            data: data.to_vec(),
            executed: false
        });
        self.transaction_count += 1;
        // self.Submission(transactionId);
        transaction_id
    }

    /*
     * Web3 call functions
     */
    /// @dev Returns number of confirmations of a transaction.
    /// @param transactionId Transaction ID.
    /// @return Number of confirmations.
    pub fn get_confirmation_count(&self, transaction_id:&uint)->uint
    {
        let cc = match self.confirmations.get(transaction_id) {
            Some(dict) => {
                dict.values().fold(0 as uint, |n, value|if *value {n + 1} else {n})
            },
            None => 0
        };
        cc
    }

    /// @dev Returns total number of transactions after filers are applied.
    /// @param pending Include pending transactions.
    /// @param executed Include executed transactions.
    /// @return Total number of transactions after filters are applied.
    pub fn get_transaction_count(&self, pending:bool, executed: bool)->uint
    {
        self.transactions.values().fold(0, |n, transaction| if pending && !transaction.executed || executed && transaction.executed {n + 1} else {n})
    }

    /// @dev Returns list of owners.
    /// @return List of owner addresses.
    pub fn get_owners(&self) -> Vec<ActorId>
    {
        self.owners.clone()
    }

    /// @dev Returns array with owner addresses, which confirmed transaction.
    /// @param transactionId Transaction ID.
    /// @return Returns array of owner addresses.
    pub fn get_confirmations(&self, transaction_id:&uint) -> Vec<ActorId> 
    {
        let confirmations = match self.confirmations.get(transaction_id) {
            Some(dict) => {
                self.owners.iter().take_while(|owner| *dict.get(owner).unwrap_or(&false)).cloned().collect()
            },
            None => vec![],
        };
        confirmations
    }

    /// @dev Returns list of transaction IDs in defined range.
    /// @param from Index start position of transaction array.
    /// @param to Index end position of transaction array.
    /// @param pending Include pending transactions.
    /// @param executed Include executed transactions.
    /// @return Returns array of transaction IDs.
    pub fn get_transaction_ids(&self, from:&uint, to:&uint, pending:bool, executed:bool)->Vec<uint>
    {
        let ids:Vec<uint> = self.transactions.iter()
        .take_while(|e|e.0 >= from && e.0 < to)
        .take_while(|e|(pending && !e.1.executed)||(executed && e.1.executed))
        .map(|e|e.0).cloned().collect();
        return ids;
    }
}

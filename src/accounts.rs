use std::sync::Arc;
use tokio::sync::Mutex;

use crate::{
    error::{Error, Result},
    storage::KVStore,
    types::{Account, ClientID, Transaction, TransactionID, TxType},
};

// This account manager processes all transactions and updates the accounts
// it's generic over the storage types for the accounts and for the transactions
#[derive(Debug)]
pub struct Manager<A, T>
where
    A: KVStore<Key = ClientID, Value = Account>,
    T: KVStore<Key = TransactionID, Value = Transaction>,
{
    accounts: Arc<Mutex<A>>,
    transactions: Arc<Mutex<T>>,
}

impl<A, T> Manager<A, T>
where
    A: KVStore<Key = ClientID, Value = Account>,
    T: KVStore<Key = TransactionID, Value = Transaction>,
{
    pub fn new(account_store: Arc<Mutex<A>>, tx_store: Arc<Mutex<T>>) -> Self {
        Self {
            accounts: account_store,
            transactions: tx_store,
        }
    }

    // process_transaction implements the main business logic of this application
    pub async fn process_transaction(&mut self, tx: Transaction) -> Result<()> {
        let mut account = self.get_account(tx.client).await?;
        if account.locked {
            return Err(Error::AccountLocked);
        }
        match tx.type_ {
            // Deposit -> add the amount to the balance
            TxType::Deposit => {
                if let Some(amount) = tx.amount {
                    account.available += amount;
                    account.total += amount;
                }
            }

            // Withdraw -> subtract the amount from the balance
            TxType::Withdrawal => {
                if let Some(amount) = tx.amount {
                    if account.available < amount {
                        return Err(Error::InsufficientFunds);
                    }
                    account.available -= amount;
                    account.total -= amount;
                }
            }

            // Dispute -> the referenced transaction is about to be reversed
            // if the disputed transaction is a deposit, the amount in question is freezed by moving it into the held balance
            TxType::Dispute => {
                let tx_store = self.transactions.lock().await;
                let source_tx = tx_store.get(tx.tx)?;
                if let Some(amount) = source_tx.amount {
                    // we can only held money back that is still in our system
                    if source_tx.type_ == TxType::Deposit {
                        // we can only hold back as much money as there is in the account
                        let mut amount = amount;
                        if amount > account.available {
                            amount = account.available;
                        }
                        account.held += amount;
                        account.available -= amount;
                    }
                }
            }

            // Reverse -> the dispute is resolved and the held balance is moved back into the available balance
            TxType::Resolve => {
                let tx_store = self.transactions.lock().await;
                let source_tx = tx_store.get(tx.tx)?;
                if let Some(amount) = source_tx.amount {
                    // we can release money back that is still in our system
                    if source_tx.type_ == TxType::Deposit {
                        // we can only hold back as much money as there is in the account
                        let mut amount = amount;
                        if amount > account.held {
                            amount = account.held;
                        }
                        account.held -= amount;
                        account.available += amount;
                    }
                }
            }

            // Chargeback -> the referenced transaction should be reversed
            // if the disputed transaction is a deposit, the amount in question is finally subtracted from the held balance
            // if the disputed transaction is a withdrawal, the amount in question is added to the available balance from thin air
            // (The assumption is that disputes and chargebacks are always executed in matching pairs so that no balances are created or destroyed)
            TxType::Chargeback => {
                let tx_store = self.transactions.lock().await;
                let source_tx = tx_store.get(tx.tx)?;
                if let Some(amount) = source_tx.amount {
                    // we can only held money back that is still in our system
                    if source_tx.type_ == TxType::Deposit {
                        let mut amount = amount;
                        if amount > account.held {
                            // we can only take as money as we find in the account
                            amount = account.held;
                        }
                        account.held -= amount;
                        account.total -= amount;
                        account.locked = true;
                    } else if source_tx.type_ == TxType::Withdrawal {
                        // the withdrawal should be reversed, so we increase the available amount
                        // the account is NOT locked since here the account holder is the disadvantaged party of the dispute
                        account.available += amount;
                        account.total += amount;
                    }
                }
            }
        }
        self.set_account(account).await?;
        Ok(())
    }

    // get_account returns the account for the given client id.
    // If the account does not exist, it is created and returned.
    async fn get_account(&mut self, client: ClientID) -> Result<Account> {
        let mut accounts = self.accounts.lock().await;
        match accounts.get(client) {
            Ok(account) => Ok(account.clone()),
            Err(_) => {
                let account = Account::new(client);
                accounts.set(client, account)?;
                Ok(accounts.get(client)?.clone())
            }
        }
    }

    // set_account sets the given account for the given client id to the new value.
    async fn set_account(&mut self, account: Account) -> Result<()> {
        self.accounts.lock().await.set(account.id, account)
    }
}

mod tests {

    #[tokio::test]
    async fn test_process_transaction_basic() -> Result<(), crate::error::Error> {
        use super::*;
        use crate::storage::InMemoryKVStore;
    
        let account_store = Arc::new(Mutex::new(InMemoryKVStore::<ClientID, Account>::new()?));
        let tx_store = Arc::new(Mutex::new(
            InMemoryKVStore::<TransactionID, Transaction>::new()?,
        ));

        let mut mgr = Manager::new(account_store.clone(), tx_store.clone());

        // deposit
        {
            let tx = Transaction {
                tx: 1,
                client: 1,
                type_: TxType::Deposit,
                amount: Some(100),
            };
            tx_store.lock().await.set(1, tx.clone())?;

            mgr.process_transaction(tx).await?;

            let store = account_store.lock().await;
            let account = store.get(1)?;
            assert_eq!(account.available, 100);
            assert_eq!(account.total, 100);
            assert_eq!(account.held, 0);
            assert_eq!(account.locked, false);
        }

        // withdrawal
        {
            let tx = Transaction {
                tx: 2,
                client: 1,
                type_: TxType::Withdrawal,
                amount: Some(50),
            };
            tx_store.lock().await.set(2, tx.clone())?;

            mgr.process_transaction(tx).await?;

            let store = account_store.lock().await;
            let account = store.get(1)?;
            assert_eq!(account.available, 50);
            assert_eq!(account.total, 50);
            assert_eq!(account.held, 0);
            assert_eq!(account.locked, false);
        }

        // dispute
        {
            let tx = Transaction {
                tx: 1,
                client: 1,
                type_: TxType::Dispute,
                amount: None,
            };

            mgr.process_transaction(tx).await?;

            let store = account_store.lock().await;
            let account = store.get(1)?;
            assert_eq!(account.available, 0);
            assert_eq!(account.total, 50);
            assert_eq!(account.held, 50);
            assert_eq!(account.locked, false);
        }

        // resolve
        {
            let tx = Transaction {
                tx: 1,
                client: 1,
                type_: TxType::Resolve,
                amount: None,
            };

            mgr.process_transaction(tx).await?;

            let store = account_store.lock().await;
            let account = store.get(1)?;
            assert_eq!(account.available, 50);
            assert_eq!(account.total, 50);
            assert_eq!(account.held, 0);
            assert_eq!(account.locked, false);
        }

        // dispute again
        {
            let tx = Transaction {
                tx: 1,
                client: 1,
                type_: TxType::Dispute,
                amount: None,
            };

            mgr.process_transaction(tx).await?;

            let store = account_store.lock().await;
            let account = store.get(1)?;
            assert_eq!(account.available, 0);
            assert_eq!(account.total, 50);
            assert_eq!(account.held, 50);
            assert_eq!(account.locked, false);
        }

        // chargeback
        {
            let tx = Transaction {
                tx: 1,
                client: 1,
                type_: TxType::Chargeback,
                amount: None,
            };

            mgr.process_transaction(tx).await?;

            let store = account_store.lock().await;
            let account = store.get(1)?;
            assert_eq!(account.available, 0);
            assert_eq!(account.total, 0);
            assert_eq!(account.held, 0);
            assert_eq!(account.locked, true);
        }

        Ok(())
    }

    #[tokio::test]
    async fn test_process_transaction_cant_withdraw_more_than_available() -> Result<()> {
        let account_store = Arc::new(Mutex::new(InMemoryKVStore::<ClientID, Account>::new()?));

        let tx_store = Arc::new(Mutex::new(
            InMemoryKVStore::<TransactionID, Transaction>::new()?,
        ));

        let mut mgr = Manager::new(account_store.clone(), tx_store.clone());

        account_store.lock().await.set(
            1,
            Account {
                id: 1,
                available: 100,
                total: 100,
                held: 0,
                locked: false,
            },
        )?;

        // withdrawal
        {
            let tx = Transaction {
                tx: 2,
                client: 1,
                type_: TxType::Withdrawal,
                amount: Some(200),
            };
            tx_store.lock().await.set(2, tx.clone())?;

            let res = mgr.process_transaction(tx).await;
            assert!(res.is_err());

            let store = account_store.lock().await;
            let account = store.get(1)?;
            assert_eq!(account.available, 100);
            assert_eq!(account.total, 100);
            assert_eq!(account.held, 0);
            assert_eq!(account.locked, false);
        }

        Ok(())
    }

    #[tokio::test]
    async fn test_process_transaction_cant_withdraw_when_account_is_locked() -> Result<()> {
        let account_store = Arc::new(Mutex::new(InMemoryKVStore::<ClientID, Account>::new()?));

        let tx_store = Arc::new(Mutex::new(
            InMemoryKVStore::<TransactionID, Transaction>::new()?,
        ));

        let mut mgr = Manager::new(account_store.clone(), tx_store.clone());

        account_store.lock().await.set(
            1,
            Account {
                id: 1,
                available: 100,
                total: 100,
                held: 0,
                locked: true,
            },
        )?;

        // withdrawal
        {
            let tx = Transaction {
                tx: 2,
                client: 1,
                type_: TxType::Withdrawal,
                amount: Some(100),
            };
            tx_store.lock().await.set(2, tx.clone())?;

            let res = mgr.process_transaction(tx).await;
            assert!(res.is_err());

            let store = account_store.lock().await;
            let account = store.get(1)?;
            assert_eq!(account.available, 100);
            assert_eq!(account.total, 100);
            assert_eq!(account.held, 0);
            assert_eq!(account.locked, true);
        }

        Ok(())
    }

    #[tokio::test]
    async fn test_process_transaction_dispute_on_deposit_holds_back_no_more_than_available(
    ) -> Result<()> {
        let account_store = Arc::new(Mutex::new(InMemoryKVStore::<ClientID, Account>::new()?));

        let tx_store = Arc::new(Mutex::new(
            InMemoryKVStore::<TransactionID, Transaction>::new()?,
        ));

        let mut mgr = Manager::new(account_store.clone(), tx_store.clone());

        account_store.lock().await.set(
            1,
            Account {
                id: 1,
                available: 50,
                total: 50,
                held: 0,
                locked: false,
            },
        )?;

        tx_store.lock().await.set(
            1,
            Transaction {
                tx: 1,
                client: 1,
                type_: TxType::Deposit,
                amount: Some(100),
            },
        )?;

        // dispute
        {
            let tx = Transaction {
                tx: 1,
                client: 1,
                type_: TxType::Dispute,
                amount: None,
            };

            mgr.process_transaction(tx).await?;

            let store = account_store.lock().await;
            let account = store.get(1)?;
            assert_eq!(account.available, 0);
            assert_eq!(account.total, 50);
            assert_eq!(account.held, 50);
            assert_eq!(account.locked, false);
        }

        Ok(())
    }

    #[tokio::test]
    async fn test_process_transaction_dispute_on_withdrawal_doesnt_hold_back_anything() -> Result<()>
    {
        let account_store = Arc::new(Mutex::new(InMemoryKVStore::<ClientID, Account>::new()?));

        let tx_store = Arc::new(Mutex::new(
            InMemoryKVStore::<TransactionID, Transaction>::new()?,
        ));

        let mut mgr = Manager::new(account_store.clone(), tx_store.clone());

        account_store.lock().await.set(
            1,
            Account {
                id: 1,
                available: 50,
                total: 50,
                held: 0,
                locked: false,
            },
        )?;

        tx_store.lock().await.set(
            1,
            Transaction {
                tx: 1,
                client: 1,
                type_: TxType::Withdrawal,
                amount: Some(100),
            },
        )?;

        // dispute
        {
            let tx = Transaction {
                tx: 1,
                client: 1,
                type_: TxType::Dispute,
                amount: None,
            };

            mgr.process_transaction(tx).await?;

            let store = account_store.lock().await;
            let account = store.get(1)?;
            assert_eq!(account.available, 50);
            assert_eq!(account.total, 50);
            assert_eq!(account.held, 0);
            assert_eq!(account.locked, false);
        }

        Ok(())
    }
}

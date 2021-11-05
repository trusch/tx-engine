use std::sync::Arc;
use tokio::sync::Mutex;

use crate::{
    error::{Error, Result},
    storage::KVStore,
    types::{Account, ClientID, Transaction, TransactionID, TxType},
};

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

    pub async fn process_transaction(&mut self, tx: Transaction) -> Result<()> {
        let mut account = self.get_account(tx.client).await?;
        if account.locked {
            return Err(Error::AccountLocked);
        }
        match tx.type_ {
            TxType::Deposit => {
                if let Some(amount) = tx.amount {
                    account.available += amount;
                    account.total += amount;
                }
            }
            TxType::Withdrawal => {
                if let Some(amount) = tx.amount {
                    if account.available < amount {
                        return Err(Error::InsufficientFunds);
                    }
                    account.available -= amount;
                    account.total -= amount;
                }
            }
            TxType::Dispute => {
                let tx_store = self.transactions.lock().await;
                let source_tx = tx_store.get(tx.tx)?;
                if let Some(amount) = source_tx.amount {
                    if source_tx.type_ == TxType::Deposit {
                        // we can only held money back that is still in our system
                        account.held += amount;
                        account.available -= amount;
                    }
                }
            }
            TxType::Resolve => {
                let tx_store = self.transactions.lock().await;
                let source_tx = tx_store.get(tx.tx)?;
                if let Some(amount) = source_tx.amount {
                    if source_tx.type_ == TxType::Deposit {
                        // we can release money back that is still in our system
                        account.held -= amount;
                        account.available += amount;
                    }
                }
            }
            TxType::Chargeback => {
                let tx_store = self.transactions.lock().await;
                let source_tx = tx_store.get(tx.tx)?;
                if let Some(amount) = source_tx.amount {
                    if source_tx.type_ == TxType::Deposit {
                        // we can only held money back that is still in our system
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

    async fn get_account(&mut self, client: ClientID) -> Result<Account> {
        let mut accounts = self.accounts.lock().await;
        match accounts.get(client) {
            Ok(account) => Ok(account.clone()),
            Err(_) => {
                let mut account = Account::default();
                account.id = client;
                accounts.set(client, account)?;
                Ok(accounts.get(client)?.clone())
            }
        }
    }

    async fn set_account(&mut self, account: Account) -> Result<()>{
        self.accounts.lock().await.set(account.id, account)
    }
}

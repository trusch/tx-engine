use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub enum TxType {
    #[serde(rename = "deposit")]
    Deposit,
    #[serde(rename = "withdrawal")]
    Withdrawal,
    #[serde(rename = "dispute")]
    Dispute,
    #[serde(rename = "resolve")]
    Resolve,
    #[serde(rename = "chargeback")]
    Chargeback,
}

pub type TransactionID = u32;

pub type ClientID = u16;

// This is one transaction row as seen in the input csv file
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct TransactionRow {
    #[serde(rename = "type")]
    pub type_: TxType,
    pub client: ClientID,
    pub tx: TransactionID,
    pub amount: Option<f64>,
}

// This is one account row as seen in the output csv file
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AccountRow {
    pub id: ClientID,
    pub available: f64,
    pub held: f64,
    pub total: f64,
    pub locked: bool,
}

// This is the internal representation of transactions
// The actual amount is saved as a u64 to prevent precision loss when calculating
// the amount here is the the actual amount as seen in the csv * 10000
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Transaction {
    #[serde(rename = "type")]
    pub type_: TxType,
    pub client: ClientID,
    pub tx: TransactionID,
    pub amount: Option<u64>,
}

// This is the internal representation of accounts
// The actual amounts are saved as a u64 to prevent precision loss when calculating
// the amount here is the the actual amount as seen in the csv * 10000
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Account {
    pub id: ClientID,
    pub available: u64,
    pub held: u64,
    pub total: u64,
    pub locked: bool,
}

impl Account {
    pub fn new(id: ClientID) -> Account {
        Account {
            id,
            available: 0,
            held: 0,
            total: 0,
            locked: false,
        }
    }
}

impl Default for Account {
    fn default() -> Self {
        Self {
            id: 0,
            available: 0,
            held: 0,
            total: 0,
            locked: false,
        }
    }
}

impl From<AccountRow> for Account {
    fn from(row: AccountRow) -> Self {
        Self {
            id: row.id,
            available: (row.available * 10000f64) as u64,
            held: (row.held * 10000f64) as u64,
            total: (row.total * 10000f64) as u64,
            locked: row.locked,
        }
    }
}

impl From<TransactionRow> for Transaction {
    fn from(row: TransactionRow) -> Self {
        Self {
            type_: row.type_,
            client: row.client,
            tx: row.tx,
            amount: row.amount.map(|x| (x * 10000f64) as u64),
        }
    }
}

impl From<Transaction> for TransactionRow {
    fn from(tx: Transaction) -> Self {
        Self {
            type_: tx.type_,
            client: tx.client,
            tx: tx.tx,
            amount: tx.amount.map(|x| (x as f64) / 10000f64),
        }
    }
}

impl From<Account> for AccountRow {
    fn from(account: Account) -> Self {
        Self {
            id: account.id,
            available: account.available as f64 / 10000f64,
            held: account.held as f64 / 10000f64,
            total: account.total as f64 / 10000f64,
            locked: account.locked,
        }
    }
}

impl Default for Transaction {
    fn default() -> Self {
        Self {
            type_: TxType::Deposit,
            client: 0,
            tx: 0,
            amount: None,
        }
    }
}

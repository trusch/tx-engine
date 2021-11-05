use crossbeam::channel::bounded;
use csv_async::{AsyncReaderBuilder, AsyncSerializer, Trim};
use std::env;
use std::path::Path;
use std::sync::Arc;
use tokio::fs::File;
use tokio::sync::Mutex;
use tokio_stream::StreamExt;

mod types;
use types::*;

mod storage;
use storage::*;

mod error;
use error::{Error, Result};

mod accounts;
use accounts::Manager as AccountManager;

#[tokio::main]
async fn main() -> Result<()> {
    // Get the command line arguments
    let args: Vec<String> = env::args().collect();
    if args.len() != 2 {
        println!("Usage: {} <transaction-csv-file>", args[0]);
        return Err(Error::InvalidArguments);
    }

    // create a transaction store, this is needed to lookup transactions that are on dispute
    // this should be backed by a file based key value store, for now its in-memory (@TODO)
    let tx_store = Arc::new(Mutex::new(
        InMemoryKVStore::<TransactionID, Transaction>::new()?,
    ));

    // create a account store
    // this is ok to be backed by a in-memory store, since we can't have more than ~65k accounts
    // Note that this abstraction will introduce a not insignificant performance cost, but it would enable us to easily upgrade to a persistent store
    let account_store = Arc::new(Mutex::new(InMemoryKVStore::<ClientID, Account>::new()?));

    // create account manager which will apply transactions to accounts
    let mut account_manager = AccountManager::new(account_store.clone(), tx_store.clone());

    // create a channel to receive transactions
    let (tx, rx) = bounded(1 << 10);

    // try to open the file
    let file = File::open(Path::new(&args[1])).await?;

    // kickoff a task that reads the transactions from the csv file and puts them into the channel
    let reader_task = tokio::spawn(async move {
        // create a CSV reader
        let mut reader = AsyncReaderBuilder::new()
            .trim(Trim::All)
            .create_deserializer(file);

        // now read the records and feed them to the manager
        let mut records = reader.deserialize::<TransactionRow>();
        while let Some(v) = records.next().await {
            match v {
                Ok(v) => {
                    let v: Transaction = v.into(); // convert from f64 to u64 ro prevent loss of precision
                    match tx.send(v) {
                        Ok(_) => {}
                        Err(e) => {
                            eprintln!("Error sending transaction to manager: {}", e);
                        }
                    };
                }
                Err(e) => {
                    println!("Error reading from csv: {}", e);
                }
            }
        }
    });

    // kick off a task that reads the channel and processes the transactions
    let processing_task = tokio::spawn(async move {
        // store and process transactions
        for tx in rx {
            // store the transaction if its a deposit or withdrawal
            if tx.type_ == TxType::Deposit || tx.type_ == TxType::Withdrawal {
                match tx_store.lock().await.set(tx.tx, tx.clone()) {
                    Ok(_) => {}
                    Err(e) => {
                        eprintln!("Error storing transaction: {}", e);
                    }
                };
            }
            // update account balances
            match account_manager.process_transaction(tx).await {
                Ok(()) => {}
                Err(err) => {
                    eprintln!("{}", err)
                }
            };
        }
    });

    // wait for the reader and processing tasks to finish
    let (r1, r2) = tokio::join!(reader_task, processing_task);
    r1?;
    r2?;

    // output final account state
    let mut writer = AsyncSerializer::from_writer(tokio::io::stdout());
    let store = account_store.lock().await.clone();
    for (_, account) in store.into_iter() {
        let row: AccountRow = account.into(); // convert from u64 to f64 to present account data in final format
        match writer.serialize(row).await {
            Ok(_) => {}
            Err(e) => eprintln!("Error writing account: {}", e),
        };
    }

    Ok(())
}

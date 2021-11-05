tx-engine
=========

## Design

This app tries to do the most things asynchronously. There are two main tasks, one for reading and deserializing the transaction data, and one for processing and storing the transactions.

It was tested with about 2GB of transaction data (~100M transactions) and it finish in ~2 minutes on my laptop.
It distributes nicely on all available CPUs but the task itself is still io-bound,
so it is utilizing only ~40-50% of the available CPU over that time.

It could be optimized further, but I think this is a good baseline.

## Notes

* The internal transaction and account models are using u64 for storing amounts. These are computed by multiplying the original floating point numbers with 10000 to preserve the required precision.
* To be able to lookup transactions in the case of a dispute, we need to store all transactions. This is not a problem for the current use-case (being a toy engine), but it would be a problem in a real world application.
    * To address this, there is the `KVStore` trait which allows to store arbitrary data. For now there is just a in-memory implementation, but this abstractions allows to use any KV store, for example a file based one or even a scalable distributed service. The raw data of a transaction is around 15 Byte, so 100M transactions is around 1.4GB of memory. I think given that its fair to just keep it in memory for now.


tx-engine
=========

I tested this with about 2GB of transaction data (~100M transactions) and finished them in ~2 minutes on my laptop.
It distributes nicely on all available CPUs but the task itself is still io-bound,
so it is utilizing only ~40-50% of my available CPU over that time.

I could optimize it further, but I think this is a good baseline.

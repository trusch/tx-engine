#!/bin/bash

MAX_CLIENT_ID=5000
MAX_AMOUNT=1000

tx_types=(dispute deposit withdrawal chargeback resolve)

function get_random_tx_type() {
    rand=$[($RANDOM % ${#tx_types[@]})]
    echo ${tx_types[$rand]}
}

function get_random_client_id() {
    rand=$[1 + ($RANDOM % $MAX_CLIENT_ID)]
    echo ${rand}
}

function get_random_amount() {
    rand=$[1 + ($RANDOM % $MAX_AMOUNT)]
    echo ${rand}
}

echo "type, client, tx, amount"
for i in {1..100000}; do
    tx_type=$(get_random_tx_type)
    client_id=$(get_random_client_id)
    echo -n "${tx_type}, ${client_id}"
    if [[ "${tx_type}" == "dispute" || "${tx_type}" == "chargeback" || "${tx_type}" == "resolve" ]]; then
        echo -n ", $[1 + ($RANDOM % $i)]" # a previous tx
    else
        echo -n ", ${i}"
    fi
    echo ", $(get_random_amount)"
done


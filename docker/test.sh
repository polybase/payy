#!/usr/bin/env bash

set -euo pipefail

cd eth
yarn
yarn compile
cd ..

# Number of times to retry the test command
RETRIES=3
# Delay in seconds between retries
DELAY=5

for i in $(seq 1 $RETRIES); do
    echo "Running cargo test (attempt $i/$RETRIES)..."
    # The `if` statement below is used to handle the exit code from `cargo test`.
    # This prevents the script from exiting immediately if the test fails because of `set -e`.
    #
    # `${RELEASE:-0}` provides a default value for the RELEASE environment variable.
    # This prevents an "unbound variable" error if it's not set when `set -u` is active.
    if cargo test $(if [ "${RELEASE:-0}" = "1" ]; then echo "--release"; fi) \
        --features smirk/slow-storage-tests \
        -- \
        --skip generate_aggregate --skip generate_utxo --skip integration_test; then
        echo "cargo test passed successfully on attempt $i."
        exit 0
    fi

    # If this wasn't the last attempt, wait before trying again.
    if [ $i -lt $RETRIES ]; then
        echo "cargo test failed. Retrying in $DELAY seconds..."
        sleep $DELAY
    fi
done

# If the loop completes without a successful run, exit with an error.
echo "cargo test failed after $RETRIES attempts."
exit 1

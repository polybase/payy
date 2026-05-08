# payy-evm-client-alloy

First-party Alloy adapters for `payy-evm-client`.

```rust
use payy_evm_client::BaseClient;
use payy_evm_client_alloy::{
    alloy_raw_transaction_submitter,
    alloy_read_client,
    to_alloy_transaction,
};

let base = BaseClient::builder(network, alloy_read_client(provider.clone()))
    .raw_transaction_submitter(alloy_raw_transaction_submitter(provider.clone()))
    .build();

let client = base.with_evm_private_key(evm_private_key)?;
let prepared = client.mint(params).prepare().await?;
let tx = to_alloy_transaction(&prepared)?;
let pending = provider.send_transaction(tx).await?;
```

The same helpers are re-exported from the `payy_evm_client` crate root when the
`payy-evm-client` crate is built with its `alloy` feature.

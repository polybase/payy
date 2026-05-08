# Privacy Vault

The Privacy Vault is a key component of Payy that enables privacy whilst maintaining compatibility with existing Ethereum infrastructure.

{% hint style="info" %}
The Privacy Vault is an optional component. Users can call `PrivacyBridge` directly, and the standard private transfer receive path no longer depends on an off-chain inbox. Privacy Vault remains useful for wallet compatibility, proof construction, private balance services, and optional provider workflows.
{% endhint %}

The Privacy Vault bundles several capabilities:

1. Storing private data - private data cannot be stored onchain
2. ZK proving transactions - converts private data and signed transactions into ZK proofs that can be used without revealing the underlying data
3. Optional provider interfaces - may expose inbox / retrieval APIs for applications that want managed off-chain workflows
4. Data sharing - if you need to share data for compliance reasons, you can authorise others to access your data (this will always be opt-in - no data will be shared without your consent)

The current privacy protocol uses direct-send plus on-chain `ExternalTransfer(prefix6, txHash)` discovery for standard recipient flows. A Privacy Vault can still help manage recipient private addresses, build proofs, and operate optional provider APIs, but it is no longer required to deliver note material off-chain for ordinary private transfers. When a vault shares or stores recipient private addresses, it should use the [canonical private-address encoding](privacy-layer/private-address.md).

You can configure your Privacy Vault by adding the `vault` parameter to your RPC URL. The vault has minimal system requirements, so can be run on micro commodity hardware.

```
https://rpc.payy.link?vault=102.10.0.69
```

When you send a transaction to the RPC, the RPC checks if it needs to generate a proof to bridge your funds into the EVM layer or send a direct payment. The ZK proof ensures that the protocol rules are followed but hides the private data.

<figure><img src="../.gitbook/assets/privacy-vault-3.png" alt=""><figcaption></figcaption></figure>

If a private storage layer is configured, RPC requests to the node will use both the public and private data to respond to requests. That means, a user with both a public and private balance would see their full balance, whereas other external users would only see the public balance.

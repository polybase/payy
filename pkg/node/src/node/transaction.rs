// lint-long-file-override allow-max-lines=300
use crate::{
    Block, Error, NodeShared, Result,
    mempool::AddError,
    network::NetworkEvent,
    utxo::{validate_txn_state, verify_txn_proof},
};
use contextful::{ErrorContextExt as _, ResultContextExt as _};
use element::Element;
use ethereum_types::U64;
use node_interface::{ElementData, ElementsVecData, MintInContractIsDifferent, RpcError};
use std::{sync::Arc, time::Duration};
use tracing::{error, info, instrument};
use zk_primitives::{
    UtxoKindMessages, UtxoProof, bridged_polygon_usdc_note_kind, extract_chain_id_from_note_kind,
};

impl NodeShared {
    pub async fn submit_transaction_and_wait(&self, utxo: UtxoProof) -> Result<Arc<Block>> {
        let mint_chain_id = match utxo.kind_messages() {
            UtxoKindMessages::Mint(mint_msgs) => {
                Some(self.chain_id_for_mint_check(mint_msgs.note_kind))
            }
            _ => None,
        };

        let mut started_waiting_at_eth_block = None;
        loop {
            match self.validate_transaction(&utxo).await {
                Ok(_) => break,
                Err(err) => {
                    let mut should_wait_for_confirmation = false;
                    if let Error::Rpc(rpc_error) = &err
                        && matches!(rpc_error.source_ref(), RpcError::MintIsNotInTheContract(..))
                    {
                        let Some(chain_id) = mint_chain_id else {
                            return Err(err);
                        };

                        let rollup_contract = match self.rollup_contract_for_chain(chain_id) {
                            Some(contract) => contract,
                            None => {
                                return Err(RpcError::UnsupportedChain { chain_id }
                                    .wrap_err("rollup contract not configured for mint chain")
                                    .into());
                            }
                        };

                        let safe_eth_height_offset = match self.config.chain_config(chain_id) {
                            Some(chain) => chain.safe_eth_height_offset,
                            None => {
                                return Err(RpcError::UnsupportedChain { chain_id }
                                    .wrap_err("chain configuration not found for mint chain")
                                    .into());
                            }
                        };

                        let current_eth_block = rollup_contract
                            .client
                            .block_number_with_fast_retries()
                            .await
                            .context("fetch pending eth block number")?
                            .as_u64();
                        let started_waiting_at_eth_block =
                            *started_waiting_at_eth_block.get_or_insert(current_eth_block);

                        let waited_too_long_for_confirmation = current_eth_block
                            .saturating_sub(started_waiting_at_eth_block)
                            > safe_eth_height_offset;

                        // TODO: we could wait a little extra time and accept mints/burns
                        // that are not even valid at `latest` height yet,
                        // because they are still in eth mempool
                        if safe_eth_height_offset == 0 || waited_too_long_for_confirmation {
                            return Err(err);
                        }

                        should_wait_for_confirmation = true;
                    }

                    if !should_wait_for_confirmation {
                        return Err(err);
                    }
                }
            }

            tokio::time::sleep(Duration::from_secs(6)).await;
        }

        self.send_all(NetworkEvent::Transaction(utxo.clone())).await;

        let mut changes = Vec::new();
        for commitment in utxo
            .public_inputs
            .input_commitments
            .into_iter()
            .chain(utxo.public_inputs.output_commitments)
        {
            if commitment.is_zero() {
                continue;
            }
            if !changes.contains(&commitment) {
                changes.push(commitment);
            }
        }

        let receiver = match self.mempool.add_with_listener(utxo.hash(), utxo, changes) {
            Ok(receiver) => receiver,
            Err(AddError::Conflict(conflict)) => {
                return Err(RpcError::TxnCommitmentAlreadyPending(ElementsVecData {
                    elements: vec![conflict],
                })
                .wrap_err("mempool reports conflicting transaction commitment while submitting")
                .into());
            }
            Err(AddError::DuplicateKey) => {
                return Err(RpcError::TxnCommitmentAlreadyPending(ElementsVecData {
                    elements: vec![],
                })
                .wrap_err("duplicate transaction commitment detected while submitting")
                .into());
            }
        };

        receiver.await.expect("recv error")
    }

    pub(super) async fn validate_transaction(&self, utxo: &UtxoProof) -> Result<()> {
        if let UtxoKindMessages::Mint(mint_msgs) = utxo.kind_messages() {
            let chain_id = self.chain_id_for_mint_check(mint_msgs.note_kind);
            let rollup_contract = match self.rollup_contract_for_chain(chain_id) {
                Some(contract) => contract,
                None => {
                    return Err(RpcError::UnsupportedChain { chain_id }
                        .wrap_err("rollup contract not configured for mint chain")
                        .into());
                }
            };
            let safe_eth_height_offset = match self.config.chain_config(chain_id) {
                Some(chain) => chain.safe_eth_height_offset,
                None => {
                    return Err(RpcError::UnsupportedChain { chain_id }
                        .wrap_err("chain configuration not found for mint chain")
                        .into());
                }
            };

            let eth_block = rollup_contract
                .client
                .block_number_with_fast_retries()
                .await
                .context("fetch latest eth block number")?;

            let safe_eth_height = match eth_block.overflowing_sub(U64::from(safe_eth_height_offset))
            {
                (safe_eth_height, false) => safe_eth_height,
                // This can happen if we are running with a local hardhat node
                (_, true) => U64::from(0),
            };
            let rollup_contract_at_safe_height = rollup_contract
                .clone()
                .at_height(Some(safe_eth_height.as_u64()));

            let get_mint_res = match rollup_contract_at_safe_height
                .get_mint(&mint_msgs.mint_hash)
                .await
                .context("query rollup contract for mint details at safe height")?
            {
                Some(res) => res,
                None => {
                    return Err(RpcError::MintIsNotInTheContract(ElementData {
                        element: mint_msgs.mint_hash,
                    })
                    .wrap_err("mint note hash not found in rollup contract at safe height")
                    .into());
                }
            };

            // Check if mint is already spent
            if get_mint_res.spent {
                return Err(RpcError::MintIsAlreadySpent(ElementsVecData {
                    elements: utxo.public_inputs.output_commitments.to_vec(),
                })
                .wrap_err("mint already marked spent in rollup contract")
                .into());
            }

            // Check mint amout/kind matches the submitted utxo proof
            if get_mint_res.amount != mint_msgs.value
                || get_mint_res.note_kind != mint_msgs.note_kind
            {
                return Err(RpcError::MintInContractIsDifferent(Box::new(
                    MintInContractIsDifferent {
                        contract_value: get_mint_res.amount,
                        contract_note_kind: get_mint_res.note_kind,
                        proof_value: mint_msgs.value,
                        proof_note_kind: mint_msgs.note_kind,
                    },
                ))
                .wrap_err("mint data in contract differs from provided UTXO proof")
                .into());
            }
        }

        verify_txn_proof(&*self.bb_backend, utxo).await?;

        validate_txn_state(
            self.config.mode,
            utxo,
            self.height(),
            &self.block_store,
            &self.notes_tree.read(),
        )
    }

    fn chain_id_for_mint_check(&self, note_kind: Element) -> u64 {
        if note_kind == bridged_polygon_usdc_note_kind()
            && let Some(primary_chain_id) = self.config.primary_chain_id()
        {
            return primary_chain_id;
        }

        extract_chain_id_from_note_kind(note_kind)
    }

    #[instrument(skip(self, txn))]
    pub async fn receive_transaction(&self, txn: UtxoProof) -> Result<()> {
        info!("Received transaction");

        if let Err(err) = self.validate_transaction(&txn).await {
            error!(
                ?err,
                "Failed to validate transaction received from another node"
            );
            return Ok(());
        }

        let mut changes = Vec::new();
        for commitment in txn
            .public_inputs
            .input_commitments
            .into_iter()
            .chain(txn.public_inputs.output_commitments)
        {
            if commitment.is_zero() {
                continue;
            }
            if !changes.contains(&commitment) {
                changes.push(commitment);
            }
        }

        match self.mempool.add(txn.hash(), txn, changes) {
            Ok(()) => {}
            Err(AddError::Conflict(conflict)) => {
                return Err(RpcError::TxnCommitmentAlreadyPending(ElementsVecData {
                    elements: vec![conflict],
                })
                .wrap_err("mempool reports conflicting transaction commitment while receiving")
                .into());
            }
            Err(AddError::DuplicateKey) => {
                return Ok(());
            }
        }

        Ok(())
    }
}

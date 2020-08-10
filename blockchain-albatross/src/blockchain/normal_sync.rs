use std::cmp;
use std::cmp::Ordering;

use parking_lot::RwLockUpgradableReadGuard;

use block::{Block, BlockError, BlockType, ForkProof, MacroBody};
#[cfg(feature = "metrics")]
use blockchain_base::chain_metrics::BlockchainMetrics;
use database::{ReadTransaction, Transaction, WriteTransaction};
use hash::{Blake2bHash, Hash};
use primitives::policy;

use crate::chain_info::ChainInfo;
use crate::slots::ForkProofInfos;
use crate::{
    Blockchain, BlockchainEvent, ChainOrdering, ForkEvent, OptionalCheck, PushError, PushResult,
};

// complicated stuff
impl Blockchain {
    //
    pub fn push(&self, block: Block) -> Result<PushResult, PushError> {
        // Only one push operation at a time.
        let _push_lock = self.push_lock.lock();

        // XXX We might want to pass this as argument to this method
        let read_txn = ReadTransaction::new(&self.env);

        // Check if we already know this block.
        let hash: Blake2bHash = block.hash();
        if self
            .chain_store
            .get_chain_info(&hash, false, Some(&read_txn))
            .is_some()
        {
            return Ok(PushResult::Known);
        }

        // Check (sort of) intrinsic block invariants.
        if let Err(e) = block.verify(self.network_id) {
            warn!("Rejecting block - verification failed ({:?})", e);
            return Err(PushError::InvalidBlock(e));
        }

        let prev_info = if let Some(prev_info) =
            self.chain_store
                .get_chain_info(&block.parent_hash(), false, Some(&read_txn))
        {
            prev_info
        } else {
            warn!(
                "Rejecting block - unknown predecessor (#{}, current #{})",
                block.header().block_number(),
                self.state.read().main_chain.head.block_number()
            );
            #[cfg(feature = "metrics")]
            self.metrics.note_orphan_block();
            return Err(PushError::Orphan);
        };

        // We have to be careful if the previous block is on a branch!
        // In this case `get_slot_at` would result in wrong slots.
        // Luckily, Albatross has the nice property that branches can only happen through
        // view changes or forks.
        // If it is a view change, we can always decide at the intersection which one is better.
        // We never even try to push on inferior branches, so we need to check this early.
        // Forks either maintain the same view numbers (and thus the same slots)
        // or there is a difference in view numbers on the way and we can discard the inferior branch.
        // This could potentially change later on, but as forks have the same slots,
        // we can always identify the inferior branch.
        // This is also the reason why fork proofs do not change the slashed set for validator selection.
        // Here, we identify inferior branches early on and discard them.
        let chain_order = self.order_chains(&block, &prev_info, Some(&read_txn));
        if chain_order == ChainOrdering::Inferior {
            // If it is an inferior chain, we ignore it as it cannot become better at any point in time.
            info!(
                "Ignoring block - inferior chain (#{}, {})",
                block.block_number(),
                hash
            );
            return Ok(PushResult::Ignored);
        }

        let view_change_proof = match block {
            Block::Macro(_) => OptionalCheck::Skip,
            Block::Micro(ref micro_block) => micro_block
                .justification
                .as_ref()
                .expect("Missing body!")
                .view_change_proof
                .as_ref()
                .into(),
        };

        let (slot, _) = self
            .get_slot_at(block.block_number(), block.view_number(), Some(&read_txn))
            .unwrap();

        {
            let intended_slot_owner = slot.public_key().uncompress_unchecked();
            // This will also check that the type at this block number is correct and whether or not an election takes place
            if let Err(e) = self.verify_block_header(
                &block.header(),
                view_change_proof,
                &intended_slot_owner,
                Some(&read_txn),
            ) {
                warn!("Rejecting block - Bad header / justification");
                return Err(e);
            }
        }

        if let Block::Micro(ref micro_block) = block {
            let justification = match micro_block
                .justification
                .as_ref()
                .expect("Missing justification!")
                .signature
                .uncompress()
            {
                Ok(justification) => justification,
                Err(_) => {
                    warn!("Rejecting block - bad justification");
                    return Err(PushError::InvalidBlock(BlockError::InvalidJustification));
                }
            };

            let intended_slot_owner = slot.public_key().uncompress_unchecked();
            if !intended_slot_owner.verify(&micro_block.header, &justification) {
                warn!("Rejecting block - invalid justification for intended slot owner");
                debug!("Block hash: {}", micro_block.header.hash::<Blake2bHash>());
                debug!("Intended slot owner: {:?}", intended_slot_owner.compress());
                return Err(PushError::InvalidBlock(BlockError::InvalidJustification));
            }

            // Check if there are two blocks in the same slot and with the same height. Since we already
            // verified the validator for the current slot, this is enough to check for fork proofs.
            // Count the micro blocks after the last macro block.
            let mut micro_blocks: Vec<Block> =
                self.chain_store
                    .get_blocks_at(block.block_number(), false, Some(&read_txn));

            // Get the micro header from the block
            let micro_header1 = &micro_block.header;

            // Get the justification for the block. We assume that the
            // validator's signature is valid.
            let justification1 = &micro_block
                .justification
                .as_ref()
                .expect("Missing justification!")
                .signature;

            // Get the view number from the block
            let view_number = block.view_number();

            for micro_block in micro_blocks.drain(..).map(|block| block.unwrap_micro()) {
                // If there's another micro block set to this view number, we
                // notify the fork event.
                if view_number == micro_block.header.view_number {
                    let micro_header2 = micro_block.header;
                    let justification2 = micro_block
                        .justification
                        .as_ref()
                        .expect("Missing justification!")
                        .signature;

                    let proof = ForkProof {
                        header1: micro_header1.clone(),
                        header2: micro_header2,
                        justification1: justification1.clone(),
                        justification2,
                    };

                    self.fork_notifier.read().notify(ForkEvent::Detected(proof));
                }
            }

            // Validate slash inherents
            for fork_proof in &micro_block.body.as_ref().unwrap().fork_proofs {
                // NOTE: if this returns None, that means that at least the previous block doesn't exist, so that fork proof is invalid anyway.
                let (slot, _) = self
                    .get_slot_at(
                        fork_proof.header1.block_number,
                        fork_proof.header1.view_number,
                        Some(&read_txn),
                    )
                    .ok_or(PushError::InvalidSuccessor)?;

                if fork_proof
                    .verify(&slot.public_key().uncompress_unchecked())
                    .is_err()
                {
                    warn!("Rejecting block - Bad fork proof: invalid owner signature");
                    return Err(PushError::InvalidSuccessor);
                }
            }
        }

        if let Block::Macro(ref macro_block) = block {
            // Check Macro Justification
            match macro_block.justification {
                None => {
                    warn!("Rejecting block - macro block without justification");
                    return Err(PushError::InvalidBlock(BlockError::NoJustification));
                }
                Some(ref justification) => {
                    if let Err(e) = justification.verify(
                        macro_block.hash(),
                        &self.current_validators(),
                        policy::TWO_THIRD_SLOTS,
                    ) {
                        warn!(
                            "Rejecting block - macro block with bad justification: {}",
                            e
                        );
                        return Err(PushError::InvalidBlock(BlockError::InvalidJustification));
                    }
                }
            }

            // The correct construction of the extrinsics is only checked after the block's inherents are applied.

            // The macro body cannot be None.
            if let Some(ref body) = macro_block.body {
                let body_hash: Blake2bHash = body.hash();
                if body_hash != macro_block.header.body_root {
                    warn!("Rejecting block - Header body hash doesn't match real body hash");
                    return Err(PushError::InvalidBlock(BlockError::BodyHashMismatch));
                }
            }
        }

        let fork_proof_infos = ForkProofInfos::fetch(&block, &self.chain_store, Some(&read_txn))
            .map_err(|err| {
                warn!("Rejecting block - slash commit failed: {:?}", err);
                PushError::InvalidSuccessor
            })?;
        let chain_info = match ChainInfo::new(block, &prev_info, &fork_proof_infos) {
            Ok(info) => info,
            Err(err) => {
                warn!("Rejecting block - slash commit failed: {:?}", err);
                return Err(PushError::InvalidSuccessor);
            }
        };

        // Drop read transaction before calling other functions.
        drop(read_txn);

        match chain_order {
            ChainOrdering::Extend => {
                return self.extend(chain_info.head.hash(), chain_info, prev_info);
            }
            ChainOrdering::Better => {
                return self.rebranch(chain_info.head.hash(), chain_info);
            }
            ChainOrdering::Inferior => unreachable!(),
            ChainOrdering::Unknown => {} // Continue.
        }

        // Otherwise, we are creating/extending a fork. Store ChainInfo.
        debug!(
            "Creating/extending fork with block {}, block number #{}, view number {}",
            chain_info.head.hash(),
            chain_info.head.block_number(),
            chain_info.head.view_number()
        );
        let mut txn = WriteTransaction::new(&self.env);
        self.chain_store
            .put_chain_info(&mut txn, &chain_info.head.hash(), &chain_info, true);
        txn.commit();

        Ok(PushResult::Forked)
    }

    fn extend(
        &self,
        block_hash: Blake2bHash,
        mut chain_info: ChainInfo,
        mut prev_info: ChainInfo,
    ) -> Result<PushResult, PushError> {
        let mut txn = WriteTransaction::new(&self.env);
        let state = self.state.read();

        // Check transactions against TransactionCache to prevent replay.
        // XXX This is technically unnecessary for macro blocks, but it doesn't hurt either.
        if state.transaction_cache.contains_any(&chain_info.head) {
            warn!("Rejecting block - transaction already included");
            txn.abort();
            return Err(PushError::DuplicateTransaction);
        }

        // Commit block to AccountsTree.
        if let Err(e) = self.commit_accounts(
            &state,
            prev_info.head.next_view_number(),
            &mut txn,
            &chain_info,
        ) {
            warn!("Rejecting block - commit failed: {:?}", e);
            txn.abort();
            #[cfg(feature = "metrics")]
            self.metrics.note_invalid_block();
            return Err(e);
        }

        drop(state);

        // Only now can we check the macro body.
        let mut is_election_block = false;
        if let Block::Macro(ref mut macro_block) = &mut chain_info.head {
            is_election_block = macro_block.is_election_block();

            if is_election_block {
                let slots = self.next_slots(&macro_block.header.seed, Some(&txn));
                if let Some(ref block_slots) =
                    macro_block.body.as_ref().expect("Missing body!").validators
                {
                    if &slots.validator_slots != block_slots {
                        warn!("Rejecting block - Validators don't match real validators");
                        return Err(PushError::InvalidBlock(BlockError::InvalidValidators));
                    }
                } else {
                    warn!("Rejecting block - Validators missing");
                    return Err(PushError::InvalidBlock(BlockError::InvalidValidators));
                }
            }

            // The final list of slashes from the previous epoch.
            let slashed_set = chain_info.slashed_set.prev_epoch_state.clone();
            // Macro blocks which do not have an election also need to keep track of the slashed set of the current
            // epoch. Macro blocks with an election always have a current_slashed_set of None, as slashes are reset
            // on election.
            let current_slashed_set = chain_info.slashed_set.current_epoch();
            let computed_extrinsics = MacroBody::from_slashed_set(slashed_set, current_slashed_set);
            let computed_extrinsics_hash: Blake2bHash = computed_extrinsics.hash();
            if computed_extrinsics_hash != macro_block.header.body_root {
                warn!("Rejecting block - Extrinsics hash doesn't match real extrinsics hash");
                return Err(PushError::InvalidBlock(BlockError::BodyHashMismatch));
            }
        }

        chain_info.on_main_chain = true;
        prev_info.main_chain_successor = Some(chain_info.head.hash());

        self.chain_store
            .put_chain_info(&mut txn, &block_hash, &chain_info, true);
        self.chain_store.put_chain_info(
            &mut txn,
            &chain_info.head.parent_hash(),
            &prev_info,
            false,
        );
        self.chain_store.set_head(&mut txn, &block_hash);

        // Acquire write lock & commit changes.
        let mut state = self.state.write();
        state.transaction_cache.push_block(&chain_info.head);

        if let Block::Macro(ref macro_block) = chain_info.head {
            state.macro_info = chain_info.clone();
            state.macro_head_hash = block_hash.clone();

            if is_election_block {
                state.election_head = macro_block.clone();
                state.election_head_hash = block_hash.clone();

                let slots = state.current_slots.take().unwrap();
                state.previous_slots.replace(slots);

                let slot = macro_block.get_slots();
                state.current_slots.replace(slot);
            }
        }

        state.main_chain = chain_info;
        state.head_hash = block_hash.clone();
        txn.commit();

        // Give up lock before notifying.
        drop(state);

        if is_election_block {
            self.notifier
                .read()
                .notify(BlockchainEvent::EpochFinalized(block_hash));
        } else {
            self.notifier
                .read()
                .notify(BlockchainEvent::Finalized(block_hash));
        }

        Ok(PushResult::Extended)
    }

    fn rebranch(
        &self,
        block_hash: Blake2bHash,
        chain_info: ChainInfo,
    ) -> Result<PushResult, PushError> {
        debug!(
            "Rebranching to fork {}, height #{}, view number {}",
            block_hash,
            chain_info.head.block_number(),
            chain_info.head.view_number()
        );

        // Find the common ancestor between our current main chain and the fork chain.
        // Walk up the fork chain until we find a block that is part of the main chain.
        // Store the chain along the way.
        let read_txn = ReadTransaction::new(&self.env);

        let mut fork_chain: Vec<(Blake2bHash, ChainInfo)> = vec![];
        let mut current: (Blake2bHash, ChainInfo) = (block_hash, chain_info);
        while !current.1.on_main_chain {
            // A fork can't contain a macro block. We already received that macro block, thus it must be on our
            // main chain.
            assert_eq!(
                current.1.head.ty(),
                BlockType::Micro,
                "Fork contains macro block"
            );

            let prev_hash = current.1.head.parent_hash().clone();
            let prev_info = self
                .chain_store
                .get_chain_info(&prev_hash, true, Some(&read_txn))
                .expect("Corrupted store: Failed to find fork predecessor while rebranching");

            fork_chain.push(current);
            current = (prev_hash, prev_info);
        }

        debug!(
            "Found common ancestor {} at height #{}, {} blocks up",
            current.0,
            current.1.head.block_number(),
            fork_chain.len()
        );

        // Revert AccountsTree & TransactionCache to the common ancestor state.
        let mut revert_chain: Vec<(Blake2bHash, ChainInfo)> = vec![];
        let mut ancestor = current;

        let mut write_txn = WriteTransaction::new(&self.env);
        let mut cache_txn;

        let state = self.state.upgradable_read();
        cache_txn = state.transaction_cache.clone();
        // XXX Get rid of the .clone() here.
        current = (state.head_hash.clone(), state.main_chain.clone());

        // Check if ancestor is in current epoch
        if ancestor.1.head.block_number() < state.macro_info.head.block_number() {
            info!("Ancestor is in finalized epoch");
            return Err(PushError::InvalidFork);
        }

        while current.0 != ancestor.0 {
            match current.1.head {
                Block::Macro(_) => {
                    // Macro blocks are final, we can't revert across them. This should be checked before we start reverting
                    panic!("Trying to rebranch across macro block");
                }
                Block::Micro(ref micro_block) => {
                    let prev_hash = micro_block.header.parent_hash.clone();
                    let prev_info = self.chain_store
                        .get_chain_info(&prev_hash, true, Some(&read_txn))
                        .expect("Corrupted store: Failed to find main chain predecessor while rebranching");

                    self.revert_accounts(
                        &state.accounts,
                        &mut write_txn,
                        &micro_block,
                        prev_info.head.view_number(),
                    )?;

                    cache_txn.revert_block(&current.1.head);

                    assert_eq!(
                        prev_info.head.state_root(),
                        &state.accounts.hash(Some(&write_txn)),
                        "Failed to revert main chain while rebranching - inconsistent state"
                    );

                    revert_chain.push(current);
                    current = (prev_hash, prev_info);
                }
            }
        }

        // Fetch missing blocks for TransactionCache.
        assert!(cache_txn.is_empty() || cache_txn.head_hash() == ancestor.0);
        let start_hash = if cache_txn.is_empty() {
            ancestor.1.main_chain_successor.unwrap()
        } else {
            cache_txn.tail_hash()
        };
        let blocks = self.chain_store.get_blocks_backward(
            &start_hash,
            cache_txn.missing_blocks(),
            true,
            Some(&read_txn),
        );
        for block in blocks.iter() {
            cache_txn.prepend_block(block)
        }
        assert_eq!(
            cache_txn.missing_blocks(),
            policy::TRANSACTION_VALIDITY_WINDOW_ALBATROSS
                .saturating_sub(ancestor.1.head.block_number() + 1)
        );

        // Check each fork block against TransactionCache & commit to AccountsTree and SlashRegistry.
        let mut prev_view_number = ancestor.1.head.next_view_number();
        let mut fork_iter = fork_chain.iter().rev();
        while let Some(fork_block) = fork_iter.next() {
            match fork_block.1.head {
                Block::Macro(_) => unreachable!(),
                Block::Micro(ref micro_block) => {
                    let result = if !cache_txn.contains_any(&fork_block.1.head) {
                        self.commit_accounts(
                            &state,
                            prev_view_number,
                            &mut write_txn,
                            &fork_block.1,
                        )
                    } else {
                        Err(PushError::DuplicateTransaction)
                    };

                    if let Err(e) = result {
                        warn!("Failed to apply fork block while rebranching - {:?}", e);
                        write_txn.abort();

                        // Delete invalid fork blocks from store.
                        let mut write_txn = WriteTransaction::new(&self.env);
                        for block in vec![fork_block].into_iter().chain(fork_iter) {
                            self.chain_store.remove_chain_info(
                                &mut write_txn,
                                &block.0,
                                micro_block.header.block_number,
                            )
                        }
                        write_txn.commit();

                        return Err(PushError::InvalidFork);
                    }

                    cache_txn.push_block(&fork_block.1.head);
                    prev_view_number = fork_block.1.head.next_view_number();
                }
            }
        }

        // Fork looks good.
        // Drop read transaction.
        read_txn.close();

        // Acquire write lock.
        let mut state = RwLockUpgradableReadGuard::upgrade(state);

        // Unset onMainChain flag / mainChainSuccessor on the current main chain up to (excluding) the common ancestor.
        for reverted_block in revert_chain.iter_mut() {
            reverted_block.1.on_main_chain = false;
            reverted_block.1.main_chain_successor = None;
            self.chain_store.put_chain_info(
                &mut write_txn,
                &reverted_block.0,
                &reverted_block.1,
                false,
            );
        }

        // Update the mainChainSuccessor of the common ancestor block.
        ancestor.1.main_chain_successor = Some(fork_chain.last().unwrap().0.clone());
        self.chain_store
            .put_chain_info(&mut write_txn, &ancestor.0, &ancestor.1, false);

        // Set onMainChain flag / mainChainSuccessor on the fork.
        for i in (0..fork_chain.len()).rev() {
            let main_chain_successor = if i > 0 {
                Some(fork_chain[i - 1].0.clone())
            } else {
                None
            };

            let fork_block = &mut fork_chain[i];
            fork_block.1.on_main_chain = true;
            fork_block.1.main_chain_successor = main_chain_successor;

            // Include the body of the new block (at position 0).
            self.chain_store
                .put_chain_info(&mut write_txn, &fork_block.0, &fork_block.1, i == 0);
        }

        // Commit transaction & update head.
        self.chain_store.set_head(&mut write_txn, &fork_chain[0].0);
        state.transaction_cache = cache_txn;
        state.main_chain = fork_chain[0].1.clone();
        state.head_hash = fork_chain[0].0.clone();
        write_txn.commit();

        // Give up lock before notifying.
        drop(state);

        let mut reverted_blocks = Vec::with_capacity(revert_chain.len());
        for (hash, chain_info) in revert_chain.into_iter().rev() {
            reverted_blocks.push((hash, chain_info.head));
        }
        let mut adopted_blocks = Vec::with_capacity(fork_chain.len());
        for (hash, chain_info) in fork_chain.into_iter().rev() {
            adopted_blocks.push((hash, chain_info.head));
        }
        let event = BlockchainEvent::Rebranched(reverted_blocks, adopted_blocks);
        self.notifier.read().notify(event);

        Ok(PushResult::Rebranched)
    }

    /// Calculate chain ordering.
    fn order_chains(
        &self,
        block: &Block,
        prev_info: &ChainInfo,
        txn_option: Option<&Transaction>,
    ) -> ChainOrdering {
        let mut chain_order = ChainOrdering::Unknown;
        if block.parent_hash() == &self.head_hash() {
            chain_order = ChainOrdering::Extend;
        } else {
            // To compare two blocks, we need to compare the view number at the intersection.
            //   [2] - [2] - [3] - [4]
            //      \- [3] - [3] - [3]
            // The example above illustrates that you actually want to choose the lower chain,
            // since its view change happened way earlier.
            // Let's thus find the first block on the branch (not on the main chain).
            // If there is a malicious fork, it might happen that the two view numbers before
            // the branch are the same. Then, we need to follow and compare.
            let mut view_numbers = vec![block.view_number()];
            let mut current: (Blake2bHash, ChainInfo) =
                (block.hash(), ChainInfo::dummy(block.clone()));
            let mut prev: (Blake2bHash, ChainInfo) = (prev_info.head.hash(), prev_info.clone());
            while !prev.1.on_main_chain {
                // Macro blocks are final
                assert_eq!(
                    prev.1.head.ty(),
                    BlockType::Micro,
                    "Trying to rebranch across macro block"
                );

                view_numbers.push(prev.1.head.view_number());

                let prev_hash = prev.1.head.parent_hash().clone();
                let prev_info = self
                    .chain_store
                    .get_chain_info(&prev_hash, false, txn_option)
                    .expect("Corrupted store: Failed to find fork predecessor while rebranching");

                current = prev;
                prev = (prev_hash, prev_info);
            }

            // Now follow the view numbers back until you find one that differs.
            // Example:
            // [0] - [0] - [1]  *correct chain*
            //    \- [0] - [0]
            // Otherwise take the longest:
            // [0] - [0] - [1] - [0]  *correct chain*
            //    \- [0] - [1]
            let current_height = current.1.head.block_number();
            let min_height = cmp::min(self.block_number(), block.block_number());

            // Iterate over common block heights starting from right after the intersection.
            for h in current_height..=min_height {
                // Take corresponding view number from branch.
                let branch_view_number = view_numbers.pop().unwrap();
                // And calculate equivalent on main chain.
                let current_on_main_chain = self
                    .chain_store
                    .get_block_at(h, false, txn_option)
                    .expect("Corrupted store: Failed to find main chain equivalent of fork");

                // Choose better one as early as possible.
                match current_on_main_chain.view_number().cmp(&branch_view_number) {
                    Ordering::Less => {
                        chain_order = ChainOrdering::Better;
                        break;
                    }
                    Ordering::Greater => {
                        chain_order = ChainOrdering::Inferior;
                        break;
                    }
                    Ordering::Equal => {} // Continue...
                }
            }

            // If they were all equal, choose the longer one.
            if chain_order == ChainOrdering::Unknown && self.block_number() < block.block_number() {
                chain_order = ChainOrdering::Better;
            }

            info!(
                "New block is on {:?} chain with fork at #{} (current #{}.{}, new block #{}.{})",
                chain_order,
                current_height - 1,
                self.block_number(),
                self.view_number(),
                block.block_number(),
                block.view_number()
            );
        }

        chain_order
    }
}
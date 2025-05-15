#![doc = include_str!("../README.md")]
// Allow for the warning in the README.
#![allow(rustdoc::broken_intra_doc_links)]

#[macro_use]
extern crate pbc_contract_codegen;
extern crate pbc_contract_common;

use create_type_spec_derive::CreateTypeSpec;
use pbc_contract_common::address::Address;
use pbc_contract_common::context::ContractContext;
use pbc_contract_common::events::EventGroup;
use pbc_contract_common::off_chain::{OffChainContext, OffChainStorage};
use pbc_contract_common::Hash;
use read_write_rpc_derive::ReadWriteRPC;
use read_write_state_derive::ReadWriteState;

mod task_queue;

use task_queue::{EngineIndex, Task, TaskQueue};

const BUCKET_ID_COMMIT: &[u8] = b"BUCKET_ID_COMMIT";
const BUCKET_ID_UPLOAD: &[u8] = b"BUCKET_ID_UPLOAD";

/// Length of a [`Randomness`].
const LENGTH_OF_RANDOMNESS: usize = 32;

/// A piece of randomness.
type Randomness = Vec<u8>;

/// Task definition for uploading already-committed to [`Randomness`].
#[derive(ReadWriteState, CreateTypeSpec)]
struct TaskUploadRandomness {
    /// Commitments that have been committed to.
    commitments: Vec<Hash>,
}

/// Task definition for committing to some [`Randomness`].
#[derive(ReadWriteState, CreateTypeSpec)]
struct TaskCommitToRandomness {}

impl Task<TaskUploadRandomness, Randomness> {
    /// Reconstructs the [`Randomness`] from the shares.
    fn reconstruct(self) -> Option<Randomness> {
        let mut result = vec![0; LENGTH_OF_RANDOMNESS];
        for share in self.all_completion_data()? {
            result = xor_bytes(&result, &share);
        }
        Some(result)
    }
}

/// XORs all bytes in the two given [`Randomness`] values.
fn xor_bytes(a: &Randomness, b: &Randomness) -> Randomness {
    a.iter().zip(b.iter()).map(|(x, y)| x ^ y).collect()
}

/// Engine configuration
#[derive(ReadWriteState, ReadWriteRPC, CreateTypeSpec, Debug)]
pub struct EngineConfig {
    /// Blockchain address of the engine. Used by contract to verify transactions.
    address: Address,
    /// HTTP endpoint of the engine. Used by users to find endpoint to interact with engines by.
    endpoint: String,
}

/// State of the contract.
#[state]
pub struct ContractState {
    /// Engine configurations
    engines: Vec<EngineConfig>,
    commit_queue: TaskQueue<TaskCommitToRandomness, Hash>,
    upload_queue: TaskQueue<TaskUploadRandomness, Randomness>,
}

impl ContractState {
    /// Engine index for the given [`Address`].
    fn engine_index(&self, addr: &Address) -> Option<EngineIndex> {
        for engine_index in 0..self.engines.len() {
            let address = self.engines.get(engine_index).map(|c| c.address);
            match address {
                Some(address) if &address == addr => {
                    return Some(engine_index as EngineIndex);
                }
                _ => {}
            }
        }
        None
    }

    fn start_generating_more_randomness(&mut self) {
        self.commit_queue.push_task(TaskCommitToRandomness {})
    }

    /// Get the reconstructed [`Randomness`] value if available.
    fn get_reconstructed_randomness(&mut self) -> Option<Randomness> {
        self.upload_queue
            .get_task(self.upload_queue.task_id_of_current())
            .and_then(|task| task.reconstruct())
    }
}

/// Initialize contract with the given engine configurations.
///
/// ## RPC Arguments
///
/// - `engines`: Configurations for all engines that serve the contract.
#[init]
pub fn initialize(_ctx: ContractContext, engines: Vec<EngineConfig>) -> ContractState {
    let mut state = ContractState {
        commit_queue: TaskQueue::new(BUCKET_ID_COMMIT.into(), engines.len() as u32),
        upload_queue: TaskQueue::new(BUCKET_ID_UPLOAD.into(), engines.len() as u32),
        engines,
    };
    state.start_generating_more_randomness();
    state
}

/// Consumres and returns the latest piece of [`Randomness`].
///
/// ## Return Value
///
/// The [`Randomness`] generated from all engines.
#[action(shortname = 0x01)]
pub fn consume_randomness(
    _ctx: ContractContext,
    mut state: ContractState,
) -> (ContractState, Vec<EventGroup>) {
    let Some(randomness) = state.get_reconstructed_randomness() else {
        panic!("No randomness available!");
    };

    state
        .upload_queue
        .remove_task(state.upload_queue.task_id_of_current());
    state.start_generating_more_randomness();
    (state, vec![EventGroup::with_return_data(randomness)])
}

/// Commit to some [`Randomness`] in the contract.
///
/// Can only be called by engines.
///
/// ## RPC Arguments
///
/// - `commit_task_id`: Identifier of the task.
/// - `randomness_commitment`: Commitment to some randomness.
#[action(shortname = 0x02)]
pub fn commit_to_randomness(
    ctx: ContractContext,
    mut state: ContractState,
    commit_task_id: u32,
    randomness_commitment: Hash,
) -> ContractState {
    let engine_index = state
        .engine_index(&ctx.sender)
        .expect("Caller is not one of the engines");

    state
        .commit_queue
        .mark_completion(engine_index, commit_task_id, randomness_commitment);

    let task = state
        .commit_queue
        .get_task(commit_task_id)
        .expect("No such commit task");

    if let Some(commitments) = task.all_completion_data() {
        state
            .upload_queue
            .push_task(TaskUploadRandomness { commitments });
        state.commit_queue.remove_task(commit_task_id);
    }

    state
}

/// Upload [`Randomness`] to the contract.
///
/// Can only be called by engines.
///
/// ## RPC Arguments
///
/// - `task_id`: Identifier of the task.
/// - `randomness`: Randomness
#[action(shortname = 0x03)]
pub fn upload_randomness(
    ctx: ContractContext,
    mut state: ContractState,
    task_id: u32,
    randomness: Randomness,
) -> ContractState {
    let engine_index = state
        .engine_index(&ctx.sender)
        .expect("Caller is not one of the engines");

    let task = state
        .upload_queue
        .get_task(task_id)
        .expect("No such upload task");

    let commitment = &task.definition().commitments[engine_index as usize];

    assert_eq!(
        &Hash::digest(&randomness),
        commitment,
        "Uploaded randomness doesn't match commitment"
    );

    state
        .upload_queue
        .mark_completion(engine_index, task_id, randomness);
    state
}

/// Solves the off-chain tasks that are currently in the task queues.
#[off_chain_on_state_change]
pub fn off_chain_on_state_update(mut ctx: OffChainContext, state: ContractState) {
    update_commitment(&mut ctx, &state);
    update_upload(&mut ctx, &state);
}

/// Checks the on-chain state for whether there is an unresolved commitment task and solves it.
///
/// This involves generating the randomness, and then sending the commitment to the contract.
fn update_commitment(ctx: &mut OffChainContext, state: &ContractState) {
    let Some(uncompleted) = state.commit_queue.get_current_task_if_uncompleted(ctx) else {
        return;
    };

    let randomness: Randomness = ctx.get_random_bytes(LENGTH_OF_RANDOMNESS as u32);
    let commitment = Hash::digest(&randomness);
    storage_commit_to_share(ctx).insert(commitment.clone(), randomness);

    state.commit_queue.report_completion_by_shortname(
        ctx,
        uncompleted,
        commit_to_randomness::SHORTNAME,
        commitment,
    );
}

/// Checks the on-chain state for whether there is an unresolved upload task, and solves it.
///
/// This involves loading the randomness that have been committed to, and then sending it to the
/// contract.
///
/// Randomness is deleted from the off-chain afterwards.
fn update_upload(ctx: &mut OffChainContext, state: &ContractState) -> Option<()> {
    let engine_index = state.engine_index(&ctx.execution_engine_address)?;
    let uncompleted = state.upload_queue.get_current_task_if_uncompleted(ctx)?;
    let commitment: Hash = uncompleted.definition().commitments[engine_index as usize].clone();
    let randomness: Randomness = storage_commit_to_share(ctx).get(&commitment)?;

    state.upload_queue.report_completion_by_shortname(
        ctx,
        uncompleted,
        upload_randomness::SHORTNAME,
        randomness,
    );

    storage_commit_to_share(ctx).remove(&commitment);

    Some(())
}

/// Stoage for shares that have been committed to.
fn storage_commit_to_share(ctx: &mut OffChainContext) -> OffChainStorage<Hash, Vec<u8>> {
    ctx.storage(BUCKET_ID_COMMITMENTS_TO_SHARE)
}

/// Bucket id used to store the shares that have been committed to.
const BUCKET_ID_COMMITMENTS_TO_SHARE: &[u8] = b"BUCKET_ID_COMMITMENTS_TO_SHARE";

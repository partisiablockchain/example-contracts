use crate::off_chain::get_randomised_r_point;
use crate::replicated_secret_sharing::{
    open_replicated_share, scalar_from_u256, try_open_replicated_share, ReplicatedSecretShare, Sum,
};
use crate::task_queue::{TaskId, TaskQueue, WithId};
use crate::ContractState;
use crate::__PBC_IS_ZK_CONTRACT;
use create_type_spec_derive::CreateTypeSpec;
use k256::ecdsa::Signature as K256Signature;
use k256::elliptic_curve::group::GroupEncoding;
use k256::elliptic_curve::point::AffineCoordinates;
use k256::elliptic_curve::scalar::IsHigh;
use k256::elliptic_curve::PrimeField;
use k256::{AffinePoint, FieldBytes, ProjectivePoint, Scalar};
use pbc_contract_common::address::Address;
use pbc_contract_common::avl_tree_map::AvlTreeMap;
use pbc_contract_common::context::ContractContext;
use pbc_contract_common::events::EventGroup;
use pbc_contract_common::signature::Signature;
use pbc_contract_common::{Hash, PublicKey, U256};
use read_write_rpc_derive::ReadWriteRPC;
use read_write_state_derive::ReadWriteState;

/// Number of engines required for signing
pub const NUM_ENGINES: u32 = 3;

#[derive(ReadWriteState, CreateTypeSpec)]
pub struct SigningComputationState {
    /// The public key corresponding to secret shared signing key. [`None`] until initialized.
    public_key: Option<PublicKey>,
    /// The information of a signing request. The created signature will be placed here once created
    signing_information: AvlTreeMap<TaskId, SigningInformation>,
    /// Preprocess state containing information of how much preprocess material has been created or used
    preprocess_state: PreprocessState,
    /// The preprocessed data pertaining to creating a signature
    pub(crate) preprocess_information: AvlTreeMap<PreprocessId, PreProcessInformation>,
    /// Work queue for uploading engine public keys
    pub(crate) engine_public_keys_queue: TaskQueue<TaskEngineUploadPublicKey, PublicKey>,
    /// Work queue for generating the secret key and uploading the public key
    pub(crate) generate_secret_key_queue:
        TaskQueue<TaskGenerateSecretKey, ReplicatedSecretShare<EncodedCurvePoint>>,
    /// Work queue for phase 1 of 4 of creating the preprocess material.
    pub(crate) pre_prep_check_queue: TaskQueue<TaskPrePrepCheck, TaskPrePrepCheckCompletion>,
    /// Work queue for phase 2 of 4 of creating the preprocess material.
    pub(crate) prep_queue: TaskQueue<TaskPrep, TaskPrepCompletion>,
    /// Work queue for phase 3 of 4 of creating the preprocess material.
    pub(crate) mul_check_one_queue: TaskQueue<TaskMulCheckOne, TaskMulCheckOneCompletion>,
    /// Work queue for phase 4 of 4 of creating the preprocess material.
    pub(crate) mul_check_two_queue: TaskQueue<TaskMulCheckTwo, TaskMulCheckTwoCompletion>,
    /// Work queue for creating the signature.
    pub(crate) sign_queue: TaskQueue<TaskSign, TaskSignCompletion>,
}

impl SigningComputationState {
    pub fn new(preprocess_config: PreprocessConfig) -> Self {
        let mut state = Self {
            public_key: None,
            preprocess_state: PreprocessState::new(preprocess_config),
            preprocess_information: AvlTreeMap::new(),
            signing_information: AvlTreeMap::new(),

            engine_public_keys_queue: TaskQueue::new(ENGINE_PUB_KEY.into(), NUM_ENGINES),
            generate_secret_key_queue: TaskQueue::new(GENERATE_SECRET_KEY.into(), NUM_ENGINES),
            pre_prep_check_queue: TaskQueue::new(TASK_PRE_PREP_CHECK.into(), NUM_ENGINES),
            prep_queue: TaskQueue::new(TASK_PREP.into(), NUM_ENGINES),
            mul_check_one_queue: TaskQueue::new(TASK_MUL_CHECK_ONE.into(), NUM_ENGINES),
            mul_check_two_queue: TaskQueue::new(TASK_MUL_CHECK_TWO.into(), NUM_ENGINES),
            sign_queue: TaskQueue::new(TASK_SIGN.into(), NUM_ENGINES),
        };
        state
            .engine_public_keys_queue
            .push_task(TaskEngineUploadPublicKey {});
        state
    }

    /// This function controls how much preprocess material we should make
    pub fn create_new_preprocess_if_needed(&mut self) {
        let num_of_used_preprocess = self.preprocess_state.allocated_preprocess_material;
        let batch_size = self.preprocess_state.preprocess_config.batch_size;
        let num_of_started_preprocess = self.preprocess_state.created_or_queued_preprocess_material;
        let Some(num_pre_processes_to_start) =
            (self.preprocess_state.preprocess_config.num_to_preprocess + num_of_used_preprocess)
                .checked_sub(num_of_started_preprocess)
        else {
            return;
        };
        let num_batches_to_start = num_pre_processes_to_start.div_ceil(batch_size);
        self.preprocess_state.created_or_queued_preprocess_material +=
            num_batches_to_start * batch_size;

        for i in 0..num_batches_to_start {
            let next_preprocess_id = num_of_started_preprocess + 1 + i * batch_size;
            self.pre_prep_check_queue.push_task(TaskPrePrepCheck {
                preprocess_id_start: next_preprocess_id,
                batch_size,
            });
        }
    }

    pub fn sign_message(&mut self, message: Vec<u8>, sender: Address, transaction_hash: Hash) {
        if self.public_key.is_none() {
            panic!("Unable to sign. Engines haven't finished generating the key.")
        }
        let message_hash = Hash::digest(&message);
        let preprocess_id = self.preprocess_state.allocate_next_preprocess_id();
        let created_task_id = self.sign_queue.push_task(TaskSign {
            preprocess_id,
            message,
            message_hash: message_hash.clone(),
            rand_hash_delta: transaction_hash,
        });
        self.signing_information.insert(
            created_task_id,
            SigningInformation::from_signature_request(sender, message_hash),
        );
        self.create_new_preprocess_if_needed();
    }

    pub fn reset_preprocessing(&mut self) {
        self.pre_prep_check_queue.cancel_all_pending_tasks();
        self.prep_queue.cancel_all_pending_tasks();
        self.mul_check_one_queue.cancel_all_pending_tasks();
        self.mul_check_two_queue.cancel_all_pending_tasks();
        self.sign_queue.cancel_all_pending_tasks();
        self.preprocess_state.allocated_preprocess_material =
            self.preprocess_state.created_or_queued_preprocess_material;
        self.create_new_preprocess_if_needed();
    }
}

/// Status for a preprocessing value.
#[derive(ReadWriteState, ReadWriteRPC, CreateTypeSpec, Debug, Clone)]
pub enum PreprocessStatus {
    /// Still calculating
    #[discriminant(0)]
    CalculatingPreprocess {},
    /// Finished with preprocessing and completed the checks
    #[discriminant(1)]
    SuccessPreprocess {},
}

/// Status for a signature
#[derive(ReadWriteState, ReadWriteRPC, CreateTypeSpec, Debug, Clone)]
pub enum SigningStatus {
    /// Still calculating
    #[discriminant(0)]
    CalculatingSigning {},
    /// Success
    #[discriminant(1)]
    SuccessSigning {},
    /// Was unable to sign with only two shares and is waiting for the third.
    #[discriminant(2)]
    UnableToSignWithTwo {},
}

/// Information about a signing request.
#[derive(ReadWriteState, CreateTypeSpec)]
pub struct SigningInformation {
    /// The address that created the signing request.
    requesting_address: Address,
    /// The hash of the message to be signed
    message_hash: Hash,
    /// The status of the signing
    signing_status: SigningStatus,
    /// The created signature, or [`None`] if not yet signed.
    signature: Option<Signature>,
}

impl SigningInformation {
    /// Create a new [`SigningInformation`].
    fn from_signature_request(
        requesting_address: Address,
        message_hash: Hash,
    ) -> SigningInformation {
        SigningInformation {
            requesting_address,
            message_hash,
            signing_status: SigningStatus::CalculatingSigning {},
            signature: None,
        }
    }

    /// Update the [`SigningInformation`] with a signature.
    fn with_signature(self, signature: Signature) -> Self {
        Self {
            requesting_address: self.requesting_address,
            message_hash: self.message_hash,
            signing_status: SigningStatus::SuccessSigning {},
            signature: Some(signature),
        }
    }

    /// Update the [`SigningInformation`] with a failure.
    fn with_failure(self) -> Self {
        Self {
            requesting_address: self.requesting_address,
            message_hash: self.message_hash,
            signing_status: SigningStatus::UnableToSignWithTwo {},
            signature: None,
        }
    }
}

/// Information for a preprocessed signing data
#[derive(ReadWriteState, CreateTypeSpec, Debug)]
pub struct PreProcessInformation {
    /// The status for the preprocessing
    pub(crate) status: PreprocessStatus,
    /// The opened R value used to create the signature
    pub(crate) opened_r: EncodedCurvePoint,
}

impl PreProcessInformation {
    /// Update the status of the preprocess information
    fn with_status(self, status: PreprocessStatus) -> Self {
        Self {
            status,
            opened_r: self.opened_r,
        }
    }
}

/// Preprocess id
pub type PreprocessId = u32;

/// Config for how much preprocess material to create.
#[derive(ReadWriteRPC, ReadWriteState, CreateTypeSpec)]
pub struct PreprocessConfig {
    /// The number of preprocessed materials the contract tries to maintain, to allow for low
    /// latency singing.
    num_to_preprocess: u32,
    /// The number of preprocess material that is created at once.
    ///
    /// The smaller this value is the more overhead is accrued per sign request.
    /// The larger this value the longer time the preprocessing will take during which the contract
    /// is blocked from doing sign requests.
    batch_size: u32,
}

/// Information about how much the preprocess material the contract has created and used.
#[derive(ReadWriteRPC, ReadWriteState, CreateTypeSpec)]
pub struct PreprocessState {
    /// The amount of preprocess material that has been claimed by signing requests.
    allocated_preprocess_material: u32,
    /// The amount of preprocess material that the contract has started to create.
    created_or_queued_preprocess_material: u32,
    /// The preprocess config
    preprocess_config: PreprocessConfig,
}

impl PreprocessState {
    /// Create a new [`PreprocessState`] from a [`PreprocessConfig`].
    fn new(preprocess_config: PreprocessConfig) -> Self {
        Self {
            allocated_preprocess_material: 0,
            created_or_queued_preprocess_material: 0,
            preprocess_config,
        }
    }
}

impl PreprocessState {
    /// Allocate the next preprocess id that a signature request should use
    pub(crate) fn allocate_next_preprocess_id(&mut self) -> PreprocessId {
        self.allocated_preprocess_material += 1;
        self.allocated_preprocess_material
    }
}

const ENGINE_PUB_KEY: &[u8] = b"ENGINE_PUB_KEY";
/// Task definition for uploading ephemeral public keys for each engine
#[derive(ReadWriteState, CreateTypeSpec)]
pub struct TaskEngineUploadPublicKey {}

const GENERATE_SECRET_KEY: &[u8] = b"GENERATE_SECRET_KEY";
/// Task definition for generating the secret key and uploading the public key
#[derive(ReadWriteState, CreateTypeSpec)]
pub struct TaskGenerateSecretKey {
    /// The public keys for the engines
    pub(crate) engine_pub_keys: Vec<PublicKey>,
}

const TASK_SIGN: &[u8] = b"TASK_SIGN";
/// Task definition for creating the signature.
#[derive(ReadWriteState, CreateTypeSpec, Clone)]
pub struct TaskSign {
    pub(crate) preprocess_id: PreprocessId,
    /// Would prefer to only store the message hash, but [`Signature`] only supports verifying messages.
    pub(crate) message: Vec<u8>,
    pub(crate) message_hash: Hash,
    pub(crate) rand_hash_delta: Hash,
}

/// Engine completion definition for creating the signature.
#[derive(ReadWriteRPC, ReadWriteState, CreateTypeSpec, Clone)]
pub struct TaskSignCompletion {
    pub(crate) s_prime: ReplicatedSecretShare<U256>,
    pub(crate) gamma_prime: ReplicatedSecretShare<U256>,
}
/// Task definition for phase 1 of 4 of creating the preprocess material.
#[derive(ReadWriteState, CreateTypeSpec)]
pub struct TaskPrePrepCheck {
    pub(crate) preprocess_id_start: PreprocessId,
    pub(crate) batch_size: u32,
}

const TASK_PRE_PREP_CHECK: &[u8] = b"TASK_PRE_PREP_CHECK";
/// Engine completion definition for phase 1 of 4 of creating the preprocess material.
#[derive(ReadWriteRPC, ReadWriteState, CreateTypeSpec, Clone)]
pub struct TaskPrePrepCheckCompletion {
    pub(crate) h_prime_intermediate_mul: Vec<U256>,
}

const TASK_PREP: &[u8] = b"TASK_PREP";
/// Task definition for phase 2 of 4 of creating the preprocess material.
#[derive(ReadWriteState, CreateTypeSpec)]
pub struct TaskPrep {
    pub(crate) preprocess_id_start: PreprocessId,
    pub(crate) batch_size: u32,
    pub(crate) opened_h_prime_intermediate_mul: Vec<U256>,
}

/// Engine completion definition for phase 2 of 4 of creating the preprocess material.
#[derive(ReadWriteRPC, ReadWriteState, CreateTypeSpec, Clone)]
pub struct TaskPrepCompletion {
    pub(crate) gamma_mul_intermediate: Vec<U256>,
    pub(crate) gamma_prime_mul_intermediate: Vec<U256>,
    pub(crate) rho_mul_intermediate: Vec<U256>,
    pub(crate) rho_prime_mul_intermediate: Vec<U256>,
    pub(crate) r_point: Vec<ReplicatedSecretShare<EncodedCurvePoint>>,
}

const TASK_MUL_CHECK_ONE: &[u8] = b"TASK_MUL_CHECK_ONE";

/// Task definition for phase 3 of 4 of creating the preprocess material.
#[derive(ReadWriteState, CreateTypeSpec)]
pub struct TaskMulCheckOne {
    pub(crate) preprocess_id_start: PreprocessId,
    pub(crate) opened_gamma_mul_intermediate: Vec<U256>,
    pub(crate) opened_gamma_prime_mul_intermediate: Vec<U256>,
    pub(crate) batch_size: u32,
    pub(crate) opened_rho_mul_intermediate: Vec<U256>,
    pub(crate) opened_rho_prime_mul_intermediate: Vec<U256>,
}
/// Engine completion definition for phase 3 of 4 of creating the preprocess material.
#[derive(ReadWriteRPC, ReadWriteState, CreateTypeSpec, Clone)]
pub struct TaskMulCheckOneCompletion {
    pub(crate) rand: ReplicatedSecretShare<U256>,
    pub(crate) epsilon: ReplicatedSecretShare<U256>,
}

const TASK_MUL_CHECK_TWO: &[u8] = b"TASK_MUL_CHECK_TWO";
/// Task definition for phase 4 of 4 of creating the preprocess material.
#[derive(ReadWriteState, CreateTypeSpec)]
pub struct TaskMulCheckTwo {
    pub(crate) preprocess_id_start: PreprocessId,
    pub(crate) batch_size: u32,
    pub(crate) opened_rand: U256,
    pub(crate) opened_epsilon: U256,
}
/// Engine completion definition for phase 4 of 4 of creating the preprocess material.
#[derive(ReadWriteRPC, ReadWriteState, CreateTypeSpec, Clone)]
pub struct TaskMulCheckTwoCompletion {
    // Combining protocol 11 line 8 and 9 into a single message
    pub(crate) tau_mul_and_open: U256,
}

/// An encoded curve point
#[derive(ReadWriteState, CreateTypeSpec, ReadWriteRPC, Eq, PartialEq, Clone, Debug)]
pub struct EncodedCurvePoint {
    pub(crate) bytes: [u8; 33],
}

/// Upload the ephemeral public key of the engine, used to create the pseudo random generators
///
/// Can only be called by an engine
#[action(shortname = 0x02)]
pub fn upload_engine_pub_key(
    ctx: ContractContext,
    mut state: ContractState,
    task_id: TaskId,
    engine_pub_key: PublicKey,
) -> ContractState {
    let engine_index = state.assert_engine(&ctx.sender);
    state
        .signing_computation_state
        .engine_public_keys_queue
        .mark_completed_by_engine(engine_index, task_id, engine_pub_key);

    let task = state
        .signing_computation_state
        .engine_public_keys_queue
        .get_task(task_id)
        .expect("No such task");

    if let Some(pub_keys) = task.all_completion_data() {
        state
            .signing_computation_state
            .generate_secret_key_queue
            .push_task(TaskGenerateSecretKey {
                engine_pub_keys: pub_keys,
            });
    }

    state
}

/// Upload public key share of this engine's secret share of the signing key.
/// Once all engines have uploaded their share the verifying public key is reconstructed and added
/// to the state.
///
/// Can only be called by an engine
#[action(shortname = 0x03)]
pub fn upload_pub_key_share(
    ctx: ContractContext,
    mut state: ContractState,
    task_id: TaskId,
    engine_pub_key: ReplicatedSecretShare<EncodedCurvePoint>,
) -> ContractState {
    let engine_index = state.assert_engine(&ctx.sender);
    let computation_state = &mut state.signing_computation_state;
    computation_state
        .generate_secret_key_queue
        .mark_completed_by_engine(engine_index, task_id, engine_pub_key);

    let task = computation_state
        .generate_secret_key_queue
        .get_task(task_id)
        .expect("No such task");

    if let Some(pub_keys) = &task.all_completion_data() {
        let shares: Vec<&ReplicatedSecretShare<EncodedCurvePoint>> = pub_keys.iter().collect();
        let public_key = open_replicated_share(&shares).expect("Unable to open public key");
        computation_state.public_key = Some(PublicKey::from_byte_array(public_key.bytes));

        computation_state.create_new_preprocess_if_needed();
    }

    state
}

/// Report that this engine has completed phase 1 of 4 of the preprocessing protocol.
/// Calls [`pre_prep_check_complete`] once all engines have called this function.
///
/// Can only be called by an engine
#[action(shortname = 0x04)]
pub fn pre_prep_check_report(
    ctx: ContractContext,
    mut state: ContractState,
    task_id: TaskId,
    completion_data: TaskPrePrepCheckCompletion,
) -> (ContractState, Vec<EventGroup>) {
    let engine_index = state.assert_engine(&ctx.sender);
    let computation_state = &mut state.signing_computation_state;
    if !computation_state.pre_prep_check_queue.is_active(task_id) {
        panic!("Task {} is not an active task", task_id);
    }
    computation_state
        .pre_prep_check_queue
        .mark_completed_by_engine(engine_index, task_id, completion_data);

    let task = computation_state
        .pre_prep_check_queue
        .get_task(task_id)
        .expect("No such task");

    let mut event_group = EventGroup::builder();
    if task.is_complete() {
        event_group
            .call(ctx.contract_address, pre_prep_check_complete::SHORTNAME)
            .argument(task_id)
            .with_cost_from_contract(1000 + 1000 * task.definition().batch_size as u64)
            .done()
    }

    (state, vec![event_group.build()])
}

/// Automatically called when all engines have called [`pre_prep_check_report`]. Opens intermediate
/// values and starts the next phase of preprocessing.
///
/// Can only be called by the contract itself
#[action(shortname = 0x14)]
pub fn pre_prep_check_complete(
    ctx: ContractContext,
    mut state: ContractState,
    task_id: TaskId,
) -> ContractState {
    assert_eq!(
        ctx.sender, ctx.contract_address,
        "Can only be called by the contract itself"
    );
    let computation_state = &mut state.signing_computation_state;
    let task = computation_state
        .pre_prep_check_queue
        .get_task(task_id)
        .expect("No such task");

    let TaskPrePrepCheck {
        preprocess_id_start,
        batch_size,
    } = *task.definition();
    let completions = task.all_completion_data().unwrap();
    let values: Vec<Vec<&U256>> = completions
        .iter()
        .map(|completion| completion.h_prime_intermediate_mul.iter().collect())
        .collect();
    let opened_h_prime_intermediate_mul = transpose(values)
        .iter()
        .map(|elems| U256::sum(elems))
        .collect();
    computation_state.prep_queue.push_task(TaskPrep {
        preprocess_id_start,
        batch_size,
        opened_h_prime_intermediate_mul,
    });

    state
}

/// Report that this engine has completed phase 2 of 4 of the preprocessing protocol.
/// Calls [`prep_complete`] once all engines have called this function.
///
/// Can only be called by an engine
#[action(shortname = 0x05)]
pub fn prep_report(
    ctx: ContractContext,
    mut state: ContractState,
    task_id: TaskId,
    completion_data: TaskPrepCompletion,
) -> (ContractState, Vec<EventGroup>) {
    let engine_index = state.assert_engine(&ctx.sender);
    let computation_state = &mut state.signing_computation_state;
    if !computation_state.prep_queue.is_active(task_id) {
        panic!("Task {} is not an active task", task_id);
    }
    computation_state
        .prep_queue
        .mark_completed_by_engine(engine_index, task_id, completion_data);

    let task = computation_state
        .prep_queue
        .get_task(task_id)
        .expect("No such task");

    let mut event_group = EventGroup::builder();
    if task.is_complete() {
        event_group
            .call(ctx.contract_address, prep_complete::SHORTNAME)
            .argument(task_id)
            .with_cost_from_contract(1000 + 1000 * task.definition().batch_size as u64)
            .done()
    }

    (state, vec![event_group.build()])
}

/// Automatically called when all engines have called [`prep_report`]. Opens intermediate
/// values and starts the next phase of preprocessing.
///
/// Can only be called by the contract itself
#[action(shortname = 0x15)]
pub fn prep_complete(
    ctx: ContractContext,
    mut state: ContractState,
    task_id: TaskId,
) -> ContractState {
    assert_eq!(
        ctx.sender, ctx.contract_address,
        "Can only be called by the contract itself"
    );
    let computation_state = &mut state.signing_computation_state;
    let task = computation_state
        .prep_queue
        .get_task(task_id)
        .expect("No such task");

    let TaskPrep {
        preprocess_id_start,
        batch_size,
        ..
    } = *task.definition();
    let completions = task.all_completion_data().unwrap();
    let gamma_values: Vec<Vec<&U256>> = completions
        .iter()
        .map(|completion| completion.gamma_mul_intermediate.iter().collect())
        .collect();
    let opened_gamma_mul_intermediate = transpose(gamma_values)
        .iter()
        .map(|elems| U256::sum(elems))
        .collect();
    let gamma_prime_values: Vec<Vec<&U256>> = completions
        .iter()
        .map(|completion| completion.gamma_prime_mul_intermediate.iter().collect())
        .collect();
    let opened_gamma_prime_mul_intermediate = transpose(gamma_prime_values)
        .iter()
        .map(|elems| U256::sum(elems))
        .collect();
    let rho_values: Vec<Vec<&U256>> = completions
        .iter()
        .map(|completion| completion.rho_mul_intermediate.iter().collect())
        .collect();
    let opened_rho_mul_intermediate = transpose(rho_values)
        .iter()
        .map(|elems| U256::sum(elems))
        .collect();
    let rho_prime_values: Vec<Vec<&U256>> = completions
        .iter()
        .map(|completion| completion.rho_prime_mul_intermediate.iter().collect())
        .collect();
    let opened_rho_prime_mul_intermediate = transpose(rho_prime_values)
        .iter()
        .map(|elems| U256::sum(elems))
        .collect();
    let r_shares: Vec<Vec<&ReplicatedSecretShare<EncodedCurvePoint>>> = completions
        .iter()
        .map(|completion| completion.r_point.iter().collect())
        .collect();
    let opened_r: Vec<EncodedCurvePoint> = transpose(r_shares)
        .iter()
        .map(|elems| open_replicated_share(elems).expect("Unable to open R"))
        .collect();
    computation_state
        .mul_check_one_queue
        .push_task(TaskMulCheckOne {
            preprocess_id_start,
            batch_size,
            opened_gamma_mul_intermediate,
            opened_gamma_prime_mul_intermediate,
            opened_rho_mul_intermediate,
            opened_rho_prime_mul_intermediate,
        });

    opened_r.into_iter().enumerate().for_each(|(i, r)| {
        computation_state.preprocess_information.insert(
            preprocess_id_start + i as u32,
            PreProcessInformation {
                status: PreprocessStatus::CalculatingPreprocess {},
                opened_r: r,
            },
        );
    });

    state
}

/// Report that this engine has completed phase 3 of 4 of the preprocessing protocol.
/// Calls [`mul_check_one_complete`] once all engines have called this function.
///
/// Can only be called by an engine
#[action(shortname = 0x06)]
pub fn mul_check_one_report(
    ctx: ContractContext,
    mut state: ContractState,
    task_id: TaskId,
    completion_data: TaskMulCheckOneCompletion,
) -> (ContractState, Vec<EventGroup>) {
    let engine_index = state.assert_engine(&ctx.sender);
    let computation_state = &mut state.signing_computation_state;
    if !computation_state.mul_check_one_queue.is_active(task_id) {
        panic!("Task {} is not an active task", task_id);
    }
    computation_state
        .mul_check_one_queue
        .mark_completed_by_engine(engine_index, task_id, completion_data);

    let task = computation_state
        .mul_check_one_queue
        .get_task(task_id)
        .expect("No such task");

    let mut event_group = EventGroup::builder();
    if task.is_complete() {
        event_group
            .call(ctx.contract_address, mul_check_one_complete::SHORTNAME)
            .argument(task_id)
            .with_cost_from_contract(1000 + 1000 * task.definition().batch_size as u64)
            .done()
    }

    (state, vec![event_group.build()])
}

/// Automatically called when all engines have called [`mul_check_one_report`]. Opens intermediate
/// values and starts the next phase of preprocessing.
///
/// Can only be called by the contract itself
#[action(shortname = 0x16)]
pub fn mul_check_one_complete(
    ctx: ContractContext,
    mut state: ContractState,
    task_id: TaskId,
) -> ContractState {
    assert_eq!(
        ctx.sender, ctx.contract_address,
        "Can only be called by the contract itself"
    );
    let computation_state = &mut state.signing_computation_state;
    let task = computation_state
        .mul_check_one_queue
        .get_task(task_id)
        .expect("No such task");

    let TaskMulCheckOne {
        preprocess_id_start,
        batch_size,
        ..
    } = *task.definition();
    let completions = task.all_completion_data().unwrap();
    let rand: Vec<&ReplicatedSecretShare<U256>> = completions
        .iter()
        .map(|completion| &completion.rand)
        .collect();
    let opened_rand = open_replicated_share(&rand).expect("Unable to open mul check randomness");
    let epsilon: Vec<&ReplicatedSecretShare<U256>> = completions
        .iter()
        .map(|completion| &completion.epsilon)
        .collect();
    let opened_epsilon = open_replicated_share(&epsilon).expect("Unable to open epsilon");

    computation_state
        .mul_check_two_queue
        .push_task(TaskMulCheckTwo {
            preprocess_id_start,
            batch_size,
            opened_rand,
            opened_epsilon,
        });

    state
}

/// Report that this engine has completed phase 4 of 4 of the preprocessing protocol.
/// Calls [`mul_check_two_complete`] once all engines have called this function.
///
/// Can only be called by an engine
#[action(shortname = 0x07)]
pub fn mul_check_two_report(
    ctx: ContractContext,
    mut state: ContractState,
    task_id: TaskId,
    completion_data: TaskMulCheckTwoCompletion,
) -> (ContractState, Vec<EventGroup>) {
    let engine_index = state.assert_engine(&ctx.sender);
    let computation_state = &mut state.signing_computation_state;
    if !computation_state.mul_check_two_queue.is_active(task_id) {
        panic!("Task {} is not an active task", task_id);
    }
    computation_state
        .mul_check_two_queue
        .mark_completed_by_engine(engine_index, task_id, completion_data);

    let task = computation_state
        .mul_check_two_queue
        .get_task(task_id)
        .expect("No such task");

    let mut event_group = EventGroup::builder();
    if task.is_complete() {
        event_group
            .call(ctx.contract_address, mul_check_two_complete::SHORTNAME)
            .argument(task_id)
            .with_cost_from_contract(1000 + 1000 * task.definition().batch_size as u64)
            .done()
    }

    (state, vec![event_group.build()])
}

/// Automatically called when all engines have called [`mul_check_two_report`]. Checks that
/// the preprocessing has been executed honestly and marks the preprocessing value as being
/// ready for signing use.
///
/// Can only be called by the contract itself
#[action(shortname = 0x17)]
pub fn mul_check_two_complete(
    ctx: ContractContext,
    mut state: ContractState,
    task_id: TaskId,
) -> ContractState {
    assert_eq!(
        ctx.sender, ctx.contract_address,
        "Can only be called by the contract itself"
    );
    let computation_state = &mut state.signing_computation_state;
    let task = computation_state
        .mul_check_two_queue
        .get_task(task_id)
        .expect("No such task");

    let TaskMulCheckTwo {
        preprocess_id_start,
        batch_size,
        ..
    } = *task.definition();
    let completions = task.all_completion_data().unwrap();
    let opened_tau = completions
        .iter()
        .map(|com| scalar_from_u256(&com.tau_mul_and_open))
        .reduce(|acc, e| acc + e)
        .unwrap();
    if !bool::from(opened_tau.is_zero()) {
        panic!("Malicious activity detected. Failed the mul check.")
    }
    for i in 0..batch_size {
        let preprocess_id = preprocess_id_start + i;
        let pre_process_info = computation_state
            .preprocess_information
            .get(&(preprocess_id))
            .unwrap();
        computation_state.preprocess_information.insert(
            preprocess_id,
            pre_process_info.with_status(PreprocessStatus::SuccessPreprocess {}),
        );
    }

    state
}

/// Report that this engine has completed the off-chain work for signing the message. Multiple
/// tasks can be reported completed in a single call to this invocation.
///
/// For each task, it will call [`sign_complete`] once two engines have called this function for
/// the task, or when three engines have called it and the contract was unable to sign with two
/// engines.
///
/// Can only be called by an engine
#[action(shortname = 0x08)]
pub fn sign_report(
    ctx: ContractContext,
    mut state: ContractState,
    task_completions: Vec<WithId<TaskSignCompletion>>,
) -> (ContractState, Vec<EventGroup>) {
    let engine_index = state.assert_engine(&ctx.sender);

    let computation_state = &mut state.signing_computation_state;
    let mut event_group = EventGroup::builder();

    for WithId {
        id: task_id,
        value: completion_data,
    } in task_completions
    {
        computation_state.sign_queue.mark_completed_by_engine(
            engine_index,
            task_id,
            completion_data,
        );

        let task = computation_state
            .sign_queue
            .get_task(task_id)
            .expect("No such task");

        let signing_task_id = task.id();

        let completed_data = task.completion_data();
        let num_completed_data = completed_data.iter().filter(|data| data.is_some()).count();
        if num_completed_data == 2 {
            event_group
                .call(ctx.contract_address, sign_complete::SHORTNAME)
                .argument(task_id)
                .with_cost_from_contract(10000)
                .done()
        } else if num_completed_data == 3 {
            let signing_status = computation_state
                .signing_information
                .get(&signing_task_id)
                .unwrap()
                .signing_status;
            if let SigningStatus::UnableToSignWithTwo {} = signing_status {
                event_group
                    .call(ctx.contract_address, sign_complete::SHORTNAME)
                    .argument(task_id)
                    .with_cost_from_contract(10000)
                    .done();
            }
        }
    }

    (state, vec![event_group.build()])
}

/// Automatically called when enough engines have called [`sign_report`]. Tries to generate a
/// signature and validates it against the public key in the state.
///
/// If it succeeds the state is updated with the created signature.
///
/// If it fails when two nodes have uploaded shares it waits for the third node and tries again.
///
/// If it fails when three nodes have uploaded shares, it tries to create the signature with each
/// of the possible subsets of two shares available. This guarantees the signature to be created.
///
/// Can only be called by the contract itself
#[action(shortname = 0x18)]
pub fn sign_complete(
    ctx: ContractContext,
    mut state: ContractState,
    task_id: TaskId,
) -> ContractState {
    assert_eq!(
        ctx.sender, ctx.contract_address,
        "Can only be called by the contract itself"
    );
    let computation_state = &mut state.signing_computation_state;
    let task = computation_state
        .sign_queue
        .get_task(task_id)
        .expect("No such task");

    let signing_task_id = task.id();
    let preprocess_id = task.definition().preprocess_id;

    let completed_data = task.completion_data();
    let num_completed_data = completed_data.iter().filter(|data| data.is_some()).count();

    let opened_r = computation_state
        .preprocess_information
        .get(&preprocess_id)
        .unwrap()
        .opened_r;
    let signing_info = computation_state
        .signing_information
        .get(&signing_task_id)
        .unwrap();
    if let Some(signature) = try_create_signature(
        completed_data,
        task.definition(),
        &opened_r,
        computation_state.public_key.as_ref().unwrap(),
    ) {
        computation_state
            .signing_information
            .insert(signing_task_id, signing_info.with_signature(signature));
        computation_state.sign_queue.mark_completion(task_id);
    } else if num_completed_data == 3 {
        for i in 0..3 {
            let mut new_completed_data = completed_data.clone();
            new_completed_data[i] = None;
            if let Some(signature) = try_create_signature(
                &new_completed_data,
                task.definition(),
                &opened_r,
                computation_state.public_key.as_ref().unwrap(),
            ) {
                computation_state
                    .signing_information
                    .insert(signing_task_id, signing_info.with_signature(signature));
                break;
            }
        }
    } else {
        computation_state
            .signing_information
            .insert(signing_task_id, signing_info.with_failure());
    }

    state
}

/// Tries to generate a signature from received shares and checks that the generated signature
/// is valid for the given public key.
fn try_create_signature(
    completed_data: &[Option<TaskSignCompletion>],
    task_data: &TaskSign,
    opened_r: &EncodedCurvePoint,
    public_key: &PublicKey,
) -> Option<Signature> {
    let (s_prime, gamma_prime) = open_signing_shares(completed_data)?;
    let TaskSign {
        rand_hash_delta,
        message,
        ..
    } = task_data;
    let signature = create_signature(rand_hash_delta, &s_prime, &gamma_prime, opened_r);
    let is_valid_signature = signature
        .recover_public_key(message)
        .is_some_and(|recovered| &recovered == public_key);
    if is_valid_signature {
        Some(signature)
    } else {
        None
    }
}

/// Open the received shares for signing a result. Returns [`None`] if the received shares are
/// inconsistent. Otherwise, returns (s_prime, gamma_prime)
fn open_signing_shares(completed_data: &[Option<TaskSignCompletion>]) -> Option<(Scalar, Scalar)> {
    let s_prime_values: Vec<Option<&ReplicatedSecretShare<U256>>> = completed_data
        .iter()
        .map(|value| value.as_ref().map(|inner| &inner.s_prime))
        .collect();
    let s_prime = scalar_from_u256(&try_open_replicated_share(&s_prime_values)?);
    let gamma_prime_values: Vec<Option<&ReplicatedSecretShare<U256>>> = completed_data
        .iter()
        .map(|value| value.as_ref().map(|inner| &inner.gamma_prime))
        .collect();
    let gamma_prime = scalar_from_u256(&try_open_replicated_share(&gamma_prime_values)?);
    Some((s_prime, gamma_prime))
}

/// Creates the signature from the opened values.
#[allow(deprecated)]
fn create_signature(
    rand_hash_delta: &Hash,
    s_prime: &Scalar,
    gamma_prime: &Scalar,
    opened_r: &EncodedCurvePoint,
) -> Signature {
    let s = s_prime * &gamma_prime.invert().unwrap();

    let delta = Scalar::from_repr(FieldBytes::clone_from_slice(rand_hash_delta.as_ref())).unwrap();
    let r_prime = get_randomised_r_point(opened_r, &delta);
    let r_x = get_x_coord(r_prime);
    let mut rec_id: u8 = r_prime.y_is_odd().unwrap_u8();

    let normalized_s = if s.is_high().into() {
        rec_id ^= 1;
        -s
    } else {
        s
    };

    let k256_sig = K256Signature::from_scalars(r_x, normalized_s).unwrap();

    let mut signature_bytes: [u8; 65] = [0; 65];
    signature_bytes[0] = rec_id;
    signature_bytes[1..].copy_from_slice(k256_sig.to_bytes().as_slice());
    Signature::from_byte_array(signature_bytes)
}

/// Get the x-coordinate from the supplied curve point.
///
/// This will fail if the value of the x coordinate is greater than curve order, but as
/// the curve point is generated randomly through an mpc computation, the probability of this
/// happening is less than 2^{-127}
pub(crate) fn get_x_coord(r_prime: AffinePoint) -> Scalar {
    Scalar::from_repr(r_prime.x()).unwrap()
}

fn transpose<T>(v: Vec<Vec<T>>) -> Vec<Vec<T>> {
    assert!(!v.is_empty());
    let len = v[0].len();
    let mut iters: Vec<_> = v.into_iter().map(|n| n.into_iter()).collect();
    (0..len)
        .map(|_| {
            iters
                .iter_mut()
                .map(|n| n.next().unwrap())
                .collect::<Vec<T>>()
        })
        .collect()
}

impl Sum for EncodedCurvePoint {
    fn sum(values: &[&Self]) -> Self {
        let result = values
            .iter()
            .map(|v| ProjectivePoint::from_bytes(&v.bytes.into()).unwrap())
            .reduce(|acc, e| acc + e)
            .unwrap();
        Self {
            bytes: *result.to_affine().to_bytes().as_ref(),
        }
    }
}

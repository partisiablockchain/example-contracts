use crate::replicated_secret_sharing::{
    scalar_from_u256, u256_from_scalar, Prg, ReplicatedSecretShare, ReplicatedSecretSharePrg,
    ReplicatedSecretSharePrgKeys, SubSessionId, Sum,
};
use crate::signing_orchestration::{
    get_x_coord, mul_check_one_report, mul_check_two_report, pre_prep_check_report, prep_report,
    sign_report, upload_engine_pub_key, upload_pub_key_share, EncodedCurvePoint,
    PreProcessInformation, PreprocessId, PreprocessStatus, TaskEngineUploadPublicKey,
    TaskGenerateSecretKey, TaskMulCheckOne, TaskMulCheckOneCompletion, TaskMulCheckTwo,
    TaskMulCheckTwoCompletion, TaskPrePrepCheck, TaskPrePrepCheckCompletion, TaskPrep,
    TaskPrepCompletion, TaskSign, TaskSignCompletion,
};
use crate::task_queue::{EngineIndex, Task, TaskQueue};
use crate::ContractState;
use k256::ecdh::diffie_hellman;
use k256::elliptic_curve::group::GroupEncoding;
use k256::elliptic_curve::PrimeField;
use k256::sha2::Digest;
use k256::{
    sha2, AffinePoint, FieldBytes, ProjectivePoint, PublicKey as K256PublicKey, Scalar, SecretKey,
};
use pbc_contract_common::address::Address;
use pbc_contract_common::off_chain::OffChainContext;
use pbc_contract_common::{PublicKey, U256};
use pbc_traits::{ReadWriteState, WriteRPC};
use read_write_state_derive::ReadWriteState;

/// Solves the off-chain tasks that are currently in the task queues.
#[off_chain_on_state_change]
pub fn off_chain_on_state_update(ctx: OffChainContext, state: ContractState) {
    state.assert_engine(&ctx.execution_engine_address);
    let mut off_chain_dispatcher = OffChainDispatcher::new(ctx, state);
    off_chain_dispatcher.process_next_task(
        |state| &state.signing_computation_state.engine_public_keys_queue,
        upload_engine_pub_key_off_chain,
    );
    off_chain_dispatcher.process_next_task(
        |state| &state.signing_computation_state.generate_secret_key_queue,
        generate_secret_key,
    );
    off_chain_dispatcher.process_next_tasks(
        |state| &state.signing_computation_state.sign_queue,
        10,
        task_sign,
    );
    off_chain_dispatcher.process_next_task(
        |state| &state.signing_computation_state.pre_prep_check_queue,
        pre_prep_mul_check,
    );
    off_chain_dispatcher.process_next_task(
        |state| &state.signing_computation_state.prep_queue,
        prep_mul,
    );
    off_chain_dispatcher.process_next_task(
        |state| &state.signing_computation_state.mul_check_one_queue,
        mul_check_one,
    );
    off_chain_dispatcher.process_next_task(
        |state| &state.signing_computation_state.mul_check_two_queue,
        mul_check_two,
    );
}

/// Off-chain dispatcher capable of getting the next task from a queue and dispatching the task
/// to the function handling it.
struct OffChainDispatcher {
    /// Off-chain context
    ctx: OffChainContext,
    /// Contract state
    state: ContractState,
}

impl OffChainDispatcher {
    /// Create new [`OffChainDispatcher`].
    fn new(ctx: OffChainContext, state: ContractState) -> Self {
        Self { ctx, state }
    }

    /// Gets the next unhandled task from the supplied work queue. If it is [`Some`] it calls
    /// the supplied function with the task.
    pub fn process_next_task<
        TaskT: ReadWriteState,
        CompletionT: ReadWriteState + Clone + WriteRPC,
    >(
        &mut self,
        queue_from_state: fn(&ContractState) -> &TaskQueue<TaskT, CompletionT>,
        handle_task: fn(
            ctx: &mut OffChainContext,
            state: &ContractState,
            task: Task<TaskT, CompletionT>,
        ),
    ) {
        let queue = queue_from_state(&self.state);
        if let Some(uncompleted) = queue.next_unhandled(&mut self.ctx) {
            handle_task(&mut self.ctx, &self.state, uncompleted)
        };
    }

    /// Gets the next [`limit`] amount of unhandled tasks from the supplied work queue. If any
    /// amount are returned, it calls the supplied function with the tasks.
    pub fn process_next_tasks<
        TaskT: ReadWriteState,
        CompletionT: ReadWriteState + Clone + WriteRPC,
    >(
        &mut self,
        queue_from_state: fn(&ContractState) -> &TaskQueue<TaskT, CompletionT>,
        limit: usize,
        handle_tasks: fn(
            ctx: &mut OffChainContext,
            state: &ContractState,
            tasks: Vec<Task<TaskT, CompletionT>>,
        ),
    ) {
        let queue = queue_from_state(&self.state);
        let uncompleted = queue.next_multiple_unhandled(&mut self.ctx, limit);
        if !uncompleted.is_empty() {
            handle_tasks(&mut self.ctx, &self.state, uncompleted)
        }
    }
}

const ENGINE_SECRET_KEY: &[u8] = b"ENGINE_SECRET_KEY";
const PRGS: &[u8] = b"PRGS";
const SIGNING_SECRET_KEY: &[u8] = b"SIGNING_SECRET_KEY";
const PRE_PREP_CHECK_STORAGE: &[u8] = b"PRE_PREP_CHECK_STORAGE";
const PREP_STORAGE: &[u8] = b"PREP_STORAGE";
const MUL_CHECK_STORAGE: &[u8] = b"MUL_CHECK_ONE_STORAGE";
const PREPROCESSED_INFORMATION_STORAGE: &[u8] = b"PREPROCESSED_INFORMATION_STORAGE";

/// Generates an ephemeral secret key and uploads the corresponding public key to the contract.
fn upload_engine_pub_key_off_chain(
    ctx: &mut OffChainContext,
    state: &ContractState,
    task: Task<TaskEngineUploadPublicKey, PublicKey>,
) {
    let secret_key_bytes = ctx.get_random_bytes(32);
    let secret = U256::from_bytes(&secret_key_bytes).unwrap();
    ctx.storage(ENGINE_SECRET_KEY)
        .insert(Unit {}, secret.clone());

    state
        .signing_computation_state
        .engine_public_keys_queue
        .report_completion(
            ctx,
            task,
            upload_engine_pub_key::rpc,
            public_key_from_secret(&secret),
            1000,
        )
}

/// Calculates the pseudo random generators based on the other engines public key. And uses them
/// to generate the engine's signing key secret share. The corresponding public key shares are
/// uploaded to the contract.
fn generate_secret_key(
    ctx: &mut OffChainContext,
    state: &ContractState,
    task: Task<TaskGenerateSecretKey, ReplicatedSecretShare<EncodedCurvePoint>>,
) {
    let engine_index = state.assert_engine(&ctx.execution_engine_address);
    let engine_secret: U256 = ctx.storage(ENGINE_SECRET_KEY).get(&Unit {}).unwrap();

    let prg_keys = create_pseudo_random_generators_keys(
        &task.definition().engine_pub_keys,
        engine_index,
        &engine_secret,
        &ctx.contract_address,
    );

    let mut prgs = ReplicatedSecretSharePrg::new(&prg_keys, 0, SubSessionId::GenerateSecretKey);

    let signing_key_share = prgs.generate_random_share();

    ctx.storage(PRGS).insert(Unit {}, prg_keys);

    let public_key_share = ReplicatedSecretShare {
        shares: [
            mul_by_generator(&signing_key_share.shares[0]),
            mul_by_generator(&signing_key_share.shares[1]),
        ],
    };

    ctx.storage(SIGNING_SECRET_KEY)
        .insert(Unit {}, signing_key_share);

    state
        .signing_computation_state
        .generate_secret_key_queue
        .report_completion(ctx, task, upload_pub_key_share::rpc, public_key_share, 2000);
}

/// Execute phase 1 of 4 of the preprocess protocol for creating signatures, storing intermediate
/// values and uploading shares to be opened.
///
/// In this phase we compute
/// ```text
/// - [epsilon] <- Rand()
/// - [h] <- Rand()
/// - [h'] <- MulBasic([h], [epsilon])
/// ```
fn pre_prep_mul_check(
    ctx: &mut OffChainContext,
    state: &ContractState,
    task: Task<TaskPrePrepCheck, TaskPrePrepCheckCompletion>,
) {
    let TaskPrePrepCheck {
        preprocess_id_start,
        batch_size,
    } = *task.definition();

    let mut prgs = ReplicatedSecretSharePrg::new(
        &load_prgs(ctx),
        preprocess_id_start,
        SubSessionId::PrePrepCheck,
    );

    let mut h_vec = vec![];
    let mut h_prime_mul_intermediate_rand_vec = vec![];
    let mut h_prime_mul_intermediate_vec = vec![];
    let epsilon = prgs.generate_random_share();
    for _ in 0..batch_size as usize {
        let h = prgs.generate_random_share();

        let h_prime_mul_intermediate = mul_basic(&mut prgs, &h, &epsilon);
        h_vec.push(h);
        h_prime_mul_intermediate_rand_vec.push(h_prime_mul_intermediate.rand_share);
        h_prime_mul_intermediate_vec.push(h_prime_mul_intermediate.intermediate_to_open);
    }

    ctx.storage(PRE_PREP_CHECK_STORAGE).insert(
        preprocess_id_start,
        PrePrepMulCheckStorage {
            epsilon,
            h: h_vec,
            h_prime_mul_intermediate_rand: h_prime_mul_intermediate_rand_vec,
        },
    );

    state
        .signing_computation_state
        .pre_prep_check_queue
        .report_completion(
            ctx,
            task,
            pre_prep_check_report::rpc,
            TaskPrePrepCheckCompletion {
                h_prime_intermediate_mul: h_prime_mul_intermediate_vec,
            },
            1000,
        );
}

/// Load the pseudo random generators from storage
fn load_prgs(ctx: &mut OffChainContext) -> ReplicatedSecretSharePrgKeys {
    ctx.storage(PRGS).get(&Unit {}).unwrap()
}

/// Execute phase 2 of 4 of the preprocess protocol for creating signatures, storing intermediate
/// values and uploading shares to be opened.
///
/// In this phase we compute
/// ```text
/// - [k] <- Rand()
/// - ([gamma], [gamma']) <- Mul([k], [h], [h'])
/// - ([rho], [rho']) <- Mul([sk], [h], [h'])
/// - <R> <- [k] * G
/// - R <- Open(<R>)
/// ```
fn prep_mul(
    ctx: &mut OffChainContext,
    state: &ContractState,
    task: Task<TaskPrep, TaskPrepCompletion>,
) {
    let engine_index = state.assert_engine(&ctx.execution_engine_address);
    let TaskPrep {
        batch_size,
        opened_h_prime_intermediate_mul,
        preprocess_id_start,
    } = task.definition();
    let PrePrepMulCheckStorage {
        epsilon,
        h,
        h_prime_mul_intermediate_rand,
    } = ctx
        .storage(PRE_PREP_CHECK_STORAGE)
        .get(preprocess_id_start)
        .unwrap();

    let mut gamma_mul_intermediate_rand_vec = vec![];
    let mut gamma_prime_mul_intermediate_rand_vec = vec![];
    let mut rho_mul_intermediate_rand_vec = vec![];
    let mut rho_prime_mul_intermediate_rand_vec = vec![];
    let mut gamma_mul_intermediate_vec = vec![];
    let mut gamma_prime_mul_intermediate_vec = vec![];
    let mut rho_mul_intermediate_vec = vec![];
    let mut rho_prime_mul_intermediate_vec = vec![];
    let mut r_vec = vec![];
    for i in 0..*batch_size as usize {
        let h_prime = h_prime_mul_intermediate_rand[i]
            .add_const(&opened_h_prime_intermediate_mul[i], engine_index);

        let sk: ReplicatedSecretShare<U256> =
            ctx.storage(SIGNING_SECRET_KEY).get(&Unit {}).unwrap();
        let mut prgs = ReplicatedSecretSharePrg::new(
            &load_prgs(ctx),
            *preprocess_id_start,
            SubSessionId::Prep,
        );
        let k = prgs.generate_random_share();

        let (gamma_mul_intermediate, gamma_prime_mul_intermediate) =
            mul_shares(&mut prgs, &k, &h[i], &h_prime);
        let (rho_mul_intermediate, rho_prime_mul_intermediate) =
            mul_shares(&mut prgs, &sk, &h[i], &h_prime);

        let r = ReplicatedSecretShare {
            shares: [
                mul_by_generator(&k.shares[0]),
                mul_by_generator(&k.shares[1]),
            ],
        };

        gamma_mul_intermediate_vec.push(gamma_mul_intermediate.intermediate_to_open);
        gamma_mul_intermediate_rand_vec.push(gamma_mul_intermediate.rand_share);
        gamma_prime_mul_intermediate_vec.push(gamma_prime_mul_intermediate.intermediate_to_open);
        gamma_prime_mul_intermediate_rand_vec.push(gamma_prime_mul_intermediate.rand_share);
        rho_mul_intermediate_vec.push(rho_mul_intermediate.intermediate_to_open);
        rho_mul_intermediate_rand_vec.push(rho_mul_intermediate.rand_share);
        rho_prime_mul_intermediate_vec.push(rho_prime_mul_intermediate.intermediate_to_open);
        rho_prime_mul_intermediate_rand_vec.push(rho_prime_mul_intermediate.rand_share);
        r_vec.push(r);
    }

    ctx.storage(PREP_STORAGE).insert(
        *preprocess_id_start,
        PrepMulStorage {
            epsilon,
            h,
            gamma_mul_intermediate_rand: gamma_mul_intermediate_rand_vec,
            gamma_prime_mul_intermediate_rand: gamma_prime_mul_intermediate_rand_vec,
            rho_mul_intermediate_rand: rho_mul_intermediate_rand_vec,
            rho_prime_mul_intermediate_rand: rho_prime_mul_intermediate_rand_vec,
        },
    );

    let preprocess_id_copy = *preprocess_id_start;
    state
        .signing_computation_state
        .prep_queue
        .report_completion(
            ctx,
            task,
            prep_report::rpc,
            TaskPrepCompletion {
                gamma_mul_intermediate: gamma_mul_intermediate_vec,
                gamma_prime_mul_intermediate: gamma_prime_mul_intermediate_vec,
                rho_mul_intermediate: rho_mul_intermediate_vec,
                rho_prime_mul_intermediate: rho_prime_mul_intermediate_vec,
                r_point: r_vec,
            },
            1000,
        );

    ctx.storage::<PreprocessId, PrePrepMulCheckStorage>(PRE_PREP_CHECK_STORAGE)
        .remove(&preprocess_id_copy);
}

/// Executes phase 3 of 4 of the preprocess protocol for creating signatures, storing intermediate
/// values and uploading shares to be opened.
///
/// In this phase we do the first part of checking that the multiplication were done properly
/// by computing
/// ```text
/// - [r] <- Rand()
/// - r <- Open([r])
/// - epsilon <- Open([epsilon])
/// ```
fn mul_check_one(
    ctx: &mut OffChainContext,
    state: &ContractState,
    task: Task<TaskMulCheckOne, TaskMulCheckOneCompletion>,
) {
    let engine_index = state.assert_engine(&ctx.execution_engine_address);
    let TaskMulCheckOne {
        preprocess_id_start,
        batch_size,
        opened_gamma_mul_intermediate,
        opened_gamma_prime_mul_intermediate,
        opened_rho_mul_intermediate,
        opened_rho_prime_mul_intermediate,
    } = task.definition();
    let mut prgs = ReplicatedSecretSharePrg::new(
        &load_prgs(ctx),
        *preprocess_id_start,
        SubSessionId::MulCheckOne,
    );

    let PrepMulStorage {
        epsilon,
        h,
        gamma_mul_intermediate_rand,
        gamma_prime_mul_intermediate_rand,
        rho_mul_intermediate_rand,
        rho_prime_mul_intermediate_rand,
    } = ctx.storage(PREP_STORAGE).get(preprocess_id_start).unwrap();

    let mut gamma_vec = vec![];
    let mut gamma_prime_vec = vec![];
    let mut rho_vec = vec![];
    let mut rho_prime_vec = vec![];
    let rand = prgs.generate_random_share();
    for i in 0..*batch_size as usize {
        let gamma = gamma_mul_intermediate_rand[i]
            .add_const(&opened_gamma_mul_intermediate[i], engine_index);
        let gamma_prime = gamma_prime_mul_intermediate_rand[i]
            .add_const(&opened_gamma_prime_mul_intermediate[i], engine_index);
        let rho =
            rho_mul_intermediate_rand[i].add_const(&opened_rho_mul_intermediate[i], engine_index);
        let rho_prime = rho_prime_mul_intermediate_rand[i]
            .add_const(&opened_rho_prime_mul_intermediate[i], engine_index);

        gamma_vec.push(gamma);
        gamma_prime_vec.push(gamma_prime);
        rho_vec.push(rho);
        rho_prime_vec.push(rho_prime);
    }

    ctx.storage(MUL_CHECK_STORAGE).insert(
        *preprocess_id_start,
        MulCheckStorage {
            h,
            gamma: gamma_vec,
            gamma_prime: gamma_prime_vec,
            rho: rho_vec,
            rho_prime: rho_prime_vec,
        },
    );

    let preprocess_id_copy = *preprocess_id_start;
    state
        .signing_computation_state
        .mul_check_one_queue
        .report_completion(
            ctx,
            task,
            mul_check_one_report::rpc,
            TaskMulCheckOneCompletion { rand, epsilon },
            1000,
        );

    ctx.storage::<PreprocessId, PrepMulStorage>(PREP_STORAGE)
        .remove(&preprocess_id_copy);
}

/// Execute phase 4 of 4 of the preprocess protocol for creating signatures, storing intermediate
/// values and uploading shares to be opened.
///
/// In this phase we do the second part of checking that the multiplication were done properly
/// by computing
/// ```text
/// - {([z_i], [z'_i])} <- {([gamma], [gamma']), ([rho], [rho'])}
/// - [v] <- sum_i{F(r, i)*[z_i]}
/// - [nu] <- sum_i{F(r, i)*[z'_i]}
/// - [zeta] <- [nu] - epsilon * [v]
/// - [r'] <- Rand()
/// - [tau] <- MulBasic([zeta], [r'])
/// - tau <- Open([tau])
/// - reject if tau != 0
/// ```
fn mul_check_two(
    ctx: &mut OffChainContext,
    state: &ContractState,
    task: Task<TaskMulCheckTwo, TaskMulCheckTwoCompletion>,
) {
    let TaskMulCheckTwo {
        preprocess_id_start,
        opened_rand,
        opened_epsilon,
        ..
    } = task.definition();

    let MulCheckStorage {
        h,
        gamma,
        gamma_prime,
        rho,
        rho_prime,
    } = ctx
        .storage(MUL_CHECK_STORAGE)
        .get(preprocess_id_start)
        .unwrap();

    let multiplications_to_check: Vec<(
        &ReplicatedSecretShare<U256>,
        &ReplicatedSecretShare<U256>,
    )> = gamma
        .iter()
        .chain(&rho)
        .zip(gamma_prime.iter().chain(&rho_prime))
        .collect();
    let (v, nu) = mul_check_linear_combination(opened_rand, &multiplications_to_check);
    let epsilon_v = v.mul_const(opened_epsilon);
    let zeta = &nu - &epsilon_v;

    let mut prgs = ReplicatedSecretSharePrg::new(
        &load_prgs(ctx),
        *preprocess_id_start,
        SubSessionId::MulCheckTwo,
    );
    let rand_prime = prgs.generate_random_share();

    let tau = mul_basic_to_immediate_open(&mut prgs, &zeta, &rand_prime);

    h.into_iter()
        .zip(gamma)
        .zip(rho)
        .enumerate()
        .for_each(|(i, ((h, gamma), rho))| {
            let preprocess_id = preprocess_id_start + i as u32;
            ctx.storage(PREPROCESSED_INFORMATION_STORAGE).insert(
                preprocess_id,
                PreprocessedInformationStorage { h, gamma, rho },
            );
        });

    let preprocess_id_copy = *preprocess_id_start;
    state
        .signing_computation_state
        .mul_check_two_queue
        .report_completion(
            ctx,
            task,
            mul_check_two_report::rpc,
            TaskMulCheckTwoCompletion {
                tau_mul_and_open: tau,
            },
            1000,
        );
    ctx.storage::<PreprocessId, MulCheckStorage>(MUL_CHECK_STORAGE)
        .remove(&preprocess_id_copy);
}

/// Executes the off-chain work for creating a signature if the preprocess material has been
/// created and verified. Uploads shares to be opened for creating the signature on-chain.
///
/// In this phase we compute
/// ```text
/// (r_x, r_y) = R' <- R + delta * G
/// [s'] <- H(m) * [h] + r_x * [rho]
/// s' <- Open([s'])
/// gamma <- Open([gamma])
/// Output signature (s' * gamma^{-1}, r_x)
/// ```
#[allow(deprecated)]
fn task_sign(
    ctx: &mut OffChainContext,
    state: &ContractState,
    tasks: Vec<Task<TaskSign, TaskSignCompletion>>,
) {
    let mut task_completions = vec![];
    let mut used_preprocess_ids = vec![];

    for task in tasks {
        let TaskSign {
            preprocess_id,
            message_hash,
            rand_hash_delta,
            ..
        } = task.definition();

        let preprocess_information = state
            .signing_computation_state
            .preprocess_information
            .get(preprocess_id);

        if preprocess_information.is_none() {
            return;
        }

        let PreProcessInformation { status, opened_r } = preprocess_information.unwrap();

        if !status.is_success() {
            return;
        }

        let delta =
            Scalar::from_repr(FieldBytes::clone_from_slice(rand_hash_delta.as_ref())).unwrap();

        let PreprocessedInformationStorage { h, gamma, rho } = ctx
            .storage(PREPROCESSED_INFORMATION_STORAGE)
            .get(preprocess_id)
            .unwrap();

        let h_1 = scalar_from_u256(&h.shares[0]);
        let h_2 = scalar_from_u256(&h.shares[1]);
        let gamma_prime = ReplicatedSecretShare {
            shares: [
                u256_from_scalar(scalar_from_u256(&gamma.shares[0]) + delta * h_1),
                u256_from_scalar(scalar_from_u256(&gamma.shares[1]) + delta * h_2),
            ],
        };

        let r_prime = get_randomised_r_point(&opened_r, &delta);
        let r_x = get_x_coord(r_prime);

        let message_hash =
            Scalar::from_repr(FieldBytes::clone_from_slice(message_hash.as_ref())).unwrap();
        let s_prime = ReplicatedSecretShare {
            shares: [
                u256_from_scalar(message_hash * h_1 + r_x * scalar_from_u256(&rho.shares[0])),
                u256_from_scalar(message_hash * h_2 + r_x * scalar_from_u256(&rho.shares[1])),
            ],
        };

        used_preprocess_ids.push(*preprocess_id);

        task_completions.push((
            task,
            TaskSignCompletion {
                s_prime,
                gamma_prime,
            },
        ));
    }

    let cpu_gas_cost = (1000 * task_completions.len()) as u64;
    state
        .signing_computation_state
        .sign_queue
        .report_completion_batched(ctx, sign_report::rpc, task_completions, cpu_gas_cost);

    for preprocess_id in used_preprocess_ids {
        ctx.storage::<PreprocessId, PreprocessedInformationStorage>(
            PREPROCESSED_INFORMATION_STORAGE,
        )
        .remove(&preprocess_id);
    }
}

/// Intermediate storage for phase 1 to phase 2 of preprocessing
#[derive(ReadWriteState)]
struct PrePrepMulCheckStorage {
    epsilon: ReplicatedSecretShare<U256>,
    h: Vec<ReplicatedSecretShare<U256>>,
    h_prime_mul_intermediate_rand: Vec<ReplicatedSecretShare<U256>>,
}
/// Intermediate storage for phase 2 to phase 3 of preprocessing
#[derive(ReadWriteState)]
struct PrepMulStorage {
    epsilon: ReplicatedSecretShare<U256>,
    h: Vec<ReplicatedSecretShare<U256>>,
    gamma_mul_intermediate_rand: Vec<ReplicatedSecretShare<U256>>,
    gamma_prime_mul_intermediate_rand: Vec<ReplicatedSecretShare<U256>>,
    rho_mul_intermediate_rand: Vec<ReplicatedSecretShare<U256>>,
    rho_prime_mul_intermediate_rand: Vec<ReplicatedSecretShare<U256>>,
}
/// Intermediate storage for phase 3 to phase 4 of preprocessing
#[derive(ReadWriteState)]
struct MulCheckStorage {
    h: Vec<ReplicatedSecretShare<U256>>,
    gamma: Vec<ReplicatedSecretShare<U256>>,
    gamma_prime: Vec<ReplicatedSecretShare<U256>>,
    rho: Vec<ReplicatedSecretShare<U256>>,
    rho_prime: Vec<ReplicatedSecretShare<U256>>,
}
/// Storage for a completed preprocess task ready for the signing request
#[derive(ReadWriteState)]
struct PreprocessedInformationStorage {
    h: ReplicatedSecretShare<U256>,
    gamma: ReplicatedSecretShare<U256>,
    rho: ReplicatedSecretShare<U256>,
}

#[derive(ReadWriteState, Clone)]
struct Unit {}

#[allow(deprecated)]
fn mul_by_generator(scalar: &U256) -> EncodedCurvePoint {
    let k256_scalar = Scalar::from_repr(FieldBytes::clone_from_slice(scalar.as_ref())).unwrap();
    let result = AffinePoint::GENERATOR * k256_scalar;
    EncodedCurvePoint {
        bytes: *result.to_bytes().as_ref(),
    }
}

fn public_key_from_secret(secret: &U256) -> PublicKey {
    let secret_key = SecretKey::from_slice(secret.as_ref()).unwrap();
    let public_key = secret_key.public_key();
    PublicKey::from_byte_array(*public_key.as_affine().to_bytes().as_ref())
}

/// Multiply two shares together. Returns a share of a value to be opened on chain as well as a
/// random share to be stored and later used with the opened value to create the resulting
/// multiplication share.
///
/// There is no guarantee that a malicious party has not tampered with the result of a [`mul_basic`].
fn mul_basic(
    prgs: &mut ReplicatedSecretSharePrg,
    x: &ReplicatedSecretShare<U256>,
    y: &ReplicatedSecretShare<U256>,
) -> MulBasicIntermediate {
    let rand = prgs.generate_random_share();
    let zero_share = prgs.generate_zero_share();
    let rho = scalar_from_u256(&rand.shares[0]) + scalar_from_u256(&zero_share);

    let x_prime = scalar_from_u256(&x.shares[0]);
    let x_prime_prime = scalar_from_u256(&x.shares[1]);
    let y_prime = scalar_from_u256(&y.shares[0]);
    let y_prime_prime = scalar_from_u256(&y.shares[1]);

    let z = x_prime * (y_prime + y_prime_prime) + x_prime_prime * y_prime - rho;

    MulBasicIntermediate {
        intermediate_to_open: u256_from_scalar(z),
        rand_share: rand,
    }
}

/// Multiple two shares together with the intention of the result being opened immediately.
/// Same as [`mul_basic`], but forgoes the random share to mask the value with.
fn mul_basic_to_immediate_open(
    prgs: &mut ReplicatedSecretSharePrg,
    x: &ReplicatedSecretShare<U256>,
    y: &ReplicatedSecretShare<U256>,
) -> U256 {
    let zero_share = scalar_from_u256(&prgs.generate_zero_share());

    let x_prime = scalar_from_u256(&x.shares[0]);
    let x_prime_prime = scalar_from_u256(&x.shares[1]);
    let y_prime = scalar_from_u256(&y.shares[0]);
    let y_prime_prime = scalar_from_u256(&y.shares[1]);

    let z = x_prime * (y_prime + y_prime_prime) + x_prime_prime * y_prime + zero_share;

    u256_from_scalar(z)
}

/// Intermediate result of a [`mul_basic`] operation.
struct MulBasicIntermediate {
    intermediate_to_open: U256,
    rand_share: ReplicatedSecretShare<U256>,
}

/// Multiply two shares together. Also makes the multiplication with a randomised version of the
/// shares for checking later that the multiplications were done properly.
fn mul_shares(
    prgs: &mut ReplicatedSecretSharePrg,
    x: &ReplicatedSecretShare<U256>,
    y: &ReplicatedSecretShare<U256>,
    epsilon_y: &ReplicatedSecretShare<U256>,
) -> (MulBasicIntermediate, MulBasicIntermediate) {
    let normal_mul_intermediate = mul_basic(prgs, x, y);
    let randomized_mul_intermediate = mul_basic(prgs, x, epsilon_y);

    (normal_mul_intermediate, randomized_mul_intermediate)
}

/// Create the linear combinations of the results of the multiplication done during the
/// preprocessing for checking that they were done honestly.
fn mul_check_linear_combination(
    rand_seed: &U256,
    multiplications_to_check: &[(&ReplicatedSecretShare<U256>, &ReplicatedSecretShare<U256>)],
) -> (ReplicatedSecretShare<U256>, ReplicatedSecretShare<U256>) {
    let mut mul_check_prg = Prg::new(rand_seed, 0, SubSessionId::LinearCombination);
    let (a, b): (
        Vec<ReplicatedSecretShare<U256>>,
        Vec<ReplicatedSecretShare<U256>>,
    ) = multiplications_to_check
        .iter()
        .map(|(z, z_prime)| {
            let random_value = mul_check_prg.generate_random_value();
            (z.mul_const(&random_value), z_prime.mul_const(&random_value))
        })
        .unzip();

    let v = ReplicatedSecretShare::sum(&a.iter().collect::<Vec<&ReplicatedSecretShare<U256>>>());
    let nu = ReplicatedSecretShare::sum(&b.iter().collect::<Vec<&ReplicatedSecretShare<U256>>>());

    (v, nu)
}

/// Randomise the R point based on a random delta value.
pub(crate) fn get_randomised_r_point(opened_r: &EncodedCurvePoint, delta: &Scalar) -> AffinePoint {
    let r = ProjectivePoint::from_bytes(&opened_r.bytes.into()).unwrap();
    let r_prime = r + ProjectivePoint::GENERATOR * delta;
    r_prime.to_affine()
}

/// Create the pseudo random generator seeds this engine should use to generate random shares.
pub(crate) fn create_pseudo_random_generators_keys(
    engine_pub_keys: &[PublicKey],
    engine_index: EngineIndex,
    engine_secret: &U256,
    contract_address: &Address,
) -> ReplicatedSecretSharePrgKeys {
    let secret_key = SecretKey::from_slice(engine_secret.as_ref()).unwrap();

    let mut cycle = engine_pub_keys
        .iter()
        .cycle()
        .skip(engine_index as usize + 1);
    let pub_key_1 = cycle.next().unwrap();
    let pub_key_2 = cycle.next().unwrap();

    ReplicatedSecretSharePrgKeys {
        prgs: [
            create_prg(pub_key_1, &secret_key, contract_address),
            create_prg(pub_key_2, &secret_key, contract_address),
        ],
    }
}

/// Create a prg seed from a public key, the secret key and the contract address.
///
/// Contract address is mixed in to avoid a case where two nodes using the same keys for two
/// different contract deployments ends up with the same prg seed for the two deployments.
fn create_prg(public_key: &PublicKey, secret_key: &SecretKey, contract_address: &Address) -> U256 {
    let public_key = K256PublicKey::from_sec1_bytes(public_key.as_ref()).unwrap();
    let shared_secret = diffie_hellman(secret_key.to_nonzero_scalar(), public_key.as_affine());
    let shared_secret_bytes = shared_secret.raw_secret_bytes();
    let mut hasher = sha2::Sha256::new();
    hasher.update(shared_secret_bytes);
    hasher.update(contract_address);
    U256::from_bytes(hasher.finalize()).unwrap()
}

impl PreprocessStatus {
    fn is_success(&self) -> bool {
        match self {
            PreprocessStatus::CalculatingPreprocess { .. } => false,
            PreprocessStatus::SuccessPreprocess { .. } => true,
        }
    }
}

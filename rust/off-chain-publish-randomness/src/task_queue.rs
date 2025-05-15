//! Task queue system for orchestrating on-chain/off-chain work.

use create_type_spec_derive::CreateTypeSpec;
use pbc_contract_common::address::Shortname;
use pbc_contract_common::avl_tree_map::AvlTreeMap;
use pbc_contract_common::off_chain::{OffChainContext, OffChainStorage};
use pbc_traits::{ReadWriteState, WriteRPC};
use read_write_state_derive::ReadWriteState;

/// Identifier of a single [`RandomnessTask`].
pub type TaskId = u32;

/// Identifier of an engine.
pub type EngineIndex = u32;

/// Gas used to send report_completion reports.
const GAS_FOR_REPORT_COMPLETION: u64 = 10_000;

/// Task in the queue.
///
/// Tasks are only treated as completed if all engines have responded.
#[derive(ReadWriteState, CreateTypeSpec, PartialEq, Eq, Debug)]
pub struct Task<DefinitionT: ReadWriteState, CompletionT: ReadWriteState> {
    /// Identifier of the [`Task`].
    id: TaskId,
    /// Definition of the [`Task`].
    definition: DefinitionT,
    /// Completion data reported by each engine.
    completion_data: Vec<Option<CompletionT>>,
}

impl<DefinitionT: ReadWriteState, CompletionT: WriteRPC + ReadWriteState>
    Task<DefinitionT, CompletionT>
{
    /// Get all completion data or nothing.
    ///
    /// Can be used to check whether the task is completed or not, and then react to the completion
    /// data.
    pub fn all_completion_data(self) -> Option<Vec<CompletionT>> {
        let mut result = vec![];
        for share in self.completion_data {
            result.push(share?);
        }
        Some(result)
    }

    /// Check whether the task have been completed or not.
    pub fn is_complete(&self) -> bool {
        self.completion_data.iter().all(Option::is_some)
    }

    /// Get the id of the task.
    pub fn id(&self) -> TaskId {
        self.id
    }

    /// Get the definition of the task.
    pub fn definition(&self) -> &DefinitionT {
        &self.definition
    }
}

/// On-chain/off-chain task queue, for orchestrating work on off-chain engines that must be
/// registered with the on-chain upon completion.
///
/// The general flow is:
///
/// 1. Task is initialized on-chain using [`TaskQueue::push_task`].
/// 2. The off-chain component notices the task using [`TaskQueue::get_current_task_if_uncompleted`].
/// 3. The off-chain component solves the task, and triggers an notification to the on-chain using [`TaskQueue::report_completion`]
/// 4. The on-chain component receives the notification, calls [`TaskQueue::mark_completion`], and
///    performs whatever action is suitable after the completion.
///
/// The completion data system is meant for situation where
#[derive(ReadWriteState, CreateTypeSpec)]
pub struct TaskQueue<DefinitionT: ReadWriteState, CompletionT: ReadWriteState> {
    /// The bucket id used to determine the off-chain storage bucket.
    bucket_id: Vec<u8>,
    /// Total number of engines.
    num_engines: EngineIndex,
    /// The identifier of the task that is currently being processed.
    ///
    /// Used to track which task should be worked on by the off-chain engines.
    ///
    /// Zero indicates that no tasks have been created so far.
    task_id_of_current: TaskId,
    /// The identifier of the
    task_id_of_last_created: TaskId,
    /// The mapping of all currently existing tasks.
    tasks: AvlTreeMap<TaskId, Task<DefinitionT, CompletionT>>,
}

impl<DefinitionT: ReadWriteState, CompletionT: WriteRPC + ReadWriteState + Clone>
    TaskQueue<DefinitionT, CompletionT>
{
    /// Create a new [`TaskQueue`].
    ///
    /// Must be called on-chain.
    ///
    /// ## Arguments
    ///
    /// - `bucket_id`: Identifier used to access off-chain storage for storing the off-chain task status.
    /// - `num_engines`: The number of engines that must solve the task.
    pub fn new(bucket_id: Vec<u8>, num_engines: EngineIndex) -> Self {
        Self {
            bucket_id,
            num_engines,
            task_id_of_current: 0,
            task_id_of_last_created: 0,
            tasks: AvlTreeMap::new(),
        }
    }

    /// Get the task id of the current task.
    pub fn task_id_of_current(&self) -> TaskId {
        self.task_id_of_current
    }

    /// Add another task to the task queue.
    ///
    /// Must be called on-chain.
    pub fn push_task(&mut self, definition: DefinitionT) {
        self.task_id_of_last_created += 1;
        self.tasks.insert(
            self.task_id_of_last_created,
            Task {
                id: self.task_id_of_last_created,
                definition,
                completion_data: vec![None; self.num_engines as usize],
            },
        );
        self.bump_current_if_needed();
    }

    /// Get the task with the given id.
    pub fn get_task(&self, task_id: TaskId) -> Option<Task<DefinitionT, CompletionT>> {
        self.tasks.get(&task_id)
    }

    /// Get the current task if the off-chain haven't completed it.
    ///
    /// Must be called off-chain.
    pub fn get_current_task_if_uncompleted(
        &self,
        context: &mut OffChainContext,
    ) -> Option<Task<DefinitionT, CompletionT>> {
        let engine_finished_task = self
            .completion_status_storage(context)
            .get(&self.task_id_of_current())
            .is_some();

        if engine_finished_task {
            None
        } else {
            self.get_task(self.task_id_of_current())
        }
    }

    /// Remove the task with the given id.
    ///
    /// Must be called on-chain.
    pub fn remove_task(&mut self, remove_task: TaskId) {
        self.tasks.remove(&remove_task)
    }

    /// Report the completion of the task to the on-chain smart-contract.
    ///
    /// Must be called off-chain.
    ///
    /// ## Arguments
    ///
    /// - [`context`]: Context used to send the transaction to the contract.
    /// - [`task`]: The completed task.
    /// - [`shortname`]: Shortname of the action to call to inform the on-chain of the completion.
    ///   Must have two RPC arguments of types: [`TaskId`] and `CompletionT`.
    /// - [`completion`]: The completion data to send to the on-chain.
    pub fn report_completion_by_shortname(
        &self,
        context: &mut OffChainContext,
        task: Task<DefinitionT, CompletionT>,
        shortname: Shortname,
        completion: CompletionT,
    ) {
        let rpc_generator = |task_id: TaskId, completion: CompletionT| {
            let mut payload: Vec<u8> = vec![];
            shortname.rpc_write_to(&mut payload).unwrap();
            task_id.rpc_write_to(&mut payload).unwrap();
            completion.rpc_write_to(&mut payload).unwrap();
            payload
        };
        self.report_completion(context, task, rpc_generator, completion)
    }

    /// Report the completion of the task to the on-chain smart-contract.
    ///
    /// Must be called off-chain.
    ///
    /// ## Arguments
    ///
    /// - [`context`]: Context used to send the transaction to the contract.
    /// - [`task`]: The completed task.
    /// - [`rpc_generator`]: Function used to create the RPC for a call to inform the on-chain of the completion.
    /// - [`completion`]: The completion data to send to the on-chain.
    pub fn report_completion<RpcGeneratorT>(
        &self,
        context: &mut OffChainContext,
        task: Task<DefinitionT, CompletionT>,
        rpc_generator: RpcGeneratorT,
        completion: CompletionT,
    ) where
        RpcGeneratorT: FnOnce(TaskId, CompletionT) -> Vec<u8>,
    {
        context.send_transaction_to_contract(
            rpc_generator(task.id(), completion),
            GAS_FOR_REPORT_COMPLETION,
        );

        self.completion_status_storage(context)
            .insert(task.id(), task.id());
    }

    /// Marks the task as being completed by the given engine and with the given completion data.
    ///
    /// Must be called on-chain.
    pub fn mark_completion(
        &mut self,
        engine_index: EngineIndex,
        task_id: TaskId,
        completion: CompletionT,
    ) {
        let mut task = self.tasks.get(&task_id).expect("No task with given id!");
        task.completion_data[engine_index as usize] = Some(completion);
        self.tasks.insert(task_id, task);
        self.bump_current_if_needed();
    }

    /// Bumps [`TaskQueue::task_id_of_current`] to the next value, if the current task have been
    /// completed.
    ///
    /// Must be called on-chain.
    fn bump_current_if_needed(&mut self) {
        if self.is_bump_of_current_needed() {
            self.task_id_of_current = self
                .task_id_of_last_created
                .min(self.task_id_of_current + 1);
        }
    }

    /// Check whether [`TaskQueue::task_id_of_current`] should be bumped or not.
    ///
    /// Must be called on-chain.
    fn is_bump_of_current_needed(&mut self) -> bool {
        match self.tasks.get(&self.task_id_of_current) {
            None => true,
            Some(current_task) => current_task.is_complete(),
        }
    }

    /// Storage used to track the off-chain completion status of the task.
    ///
    /// Must be called off-chain.
    fn completion_status_storage(
        &self,
        context: &mut OffChainContext,
    ) -> OffChainStorage<TaskId, TaskId> {
        context.storage(&self.bucket_id)
    }
}

/// Tests for [`TaskQueue`].
#[cfg(test)]
mod tests {
    use super::*;

    #[derive(ReadWriteState, read_write_rpc_derive::WriteRPC, Clone, PartialEq, Eq, Debug)]
    struct Empty {}

    /// Can alternate between pushing and completing tasks.
    #[test]
    fn test_queue_push_complete() {
        let mut queue: TaskQueue<Empty, Empty> = TaskQueue::new(vec![1, 2, 3], 2);

        assert_eq!(queue.task_id_of_current(), 0);

        queue.push_task(Empty {});
        assert_eq!(queue.task_id_of_current(), 1);

        queue.mark_completion(0, 1, Empty {});
        queue.mark_completion(1, 1, Empty {});

        queue.push_task(Empty {});
        assert_eq!(queue.task_id_of_current(), 2);
        queue.mark_completion(0, 2, Empty {});
        queue.mark_completion(1, 2, Empty {});

        queue.push_task(Empty {});
        assert_eq!(queue.task_id_of_current(), 3);
        queue.mark_completion(0, 3, Empty {});
        queue.mark_completion(1, 3, Empty {});
        assert_eq!(queue.task_id_of_current(), 3);
    }

    /// Can push many times before beginning to complete tasks.
    #[test]
    fn test_queue_push_many_complete_many() {
        let mut queue: TaskQueue<Empty, Empty> = TaskQueue::new(vec![1, 2, 3], 2);

        assert_eq!(queue.task_id_of_current(), 0);

        queue.push_task(Empty {});
        queue.push_task(Empty {});
        queue.push_task(Empty {});

        assert_eq!(queue.task_id_of_current(), 1);

        queue.mark_completion(0, 1, Empty {});
        queue.mark_completion(1, 1, Empty {});

        assert_eq!(queue.task_id_of_current(), 2);
        queue.mark_completion(0, 2, Empty {});
        queue.mark_completion(1, 2, Empty {});

        assert_eq!(queue.task_id_of_current(), 3);
        queue.mark_completion(0, 3, Empty {});
        queue.mark_completion(1, 3, Empty {});
        assert_eq!(queue.task_id_of_current(), 3);
    }

    /// All completion data is available once all engines have been marked as completing the task.
    #[test]
    fn task_completion_data() {
        let mut queue: TaskQueue<Empty, Empty> = TaskQueue::new(vec![1, 2, 3], 2);

        assert_eq!(queue.get_task(1), None);

        queue.push_task(Empty {});

        assert_eq!(queue.get_task(1).unwrap().all_completion_data(), None);

        queue.mark_completion(0, 1, Empty {});

        assert_eq!(queue.get_task(1).unwrap().all_completion_data(), None);

        queue.mark_completion(1, 1, Empty {});

        assert_eq!(
            queue.get_task(1).unwrap().all_completion_data(),
            Some(vec![Empty {}, Empty {}])
        );
    }

    /// Tasks can be removed while current
    #[test]
    fn remove_current_task() {
        let mut queue: TaskQueue<Empty, Empty> = TaskQueue::new(vec![1, 2, 3], 2);

        queue.push_task(Empty {});
        queue.remove_task(1);
        assert_eq!(queue.task_id_of_current(), 1);

        queue.push_task(Empty {});
        queue.remove_task(2);
        assert_eq!(queue.task_id_of_current(), 2);

        queue.push_task(Empty {});
        queue.remove_task(3);
        assert_eq!(queue.task_id_of_current(), 3);

        queue.push_task(Empty {});
        assert!(queue.get_task(4).is_some());
        assert_eq!(queue.task_id_of_current(), 4);
    }
}

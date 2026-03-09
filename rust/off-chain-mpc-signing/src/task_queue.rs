//! Task queue system for orchestrating on-chain/off-chain work.
//!
//! Copied from the off-chain-publish-randomness contract, but modified to allow the off-chain to
//! process any task it has not yet completed and not just the next task that has yet to be
//! completed by all nodes.

use create_type_spec_derive::CreateTypeSpec;
use pbc_contract_common::avl_tree_map::AvlTreeMap;
use pbc_contract_common::off_chain::{OffChainContext, OffChainStorage};
use pbc_traits::ReadRPC;
use pbc_traits::{ReadWriteState, WriteRPC};
use read_write_rpc_derive::ReadWriteRPC;
use read_write_state_derive::ReadWriteState;

/// Identifier of a single [`RandomnessTask`].
pub type TaskId = u32;

/// Identifier of an engine.
pub type EngineIndex = u32;

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

    /// Get the definition of the task.
    pub fn completion_data(&self) -> &Vec<Option<CompletionT>> {
        &self.completion_data
    }
}

/// On-chain/off-chain task queue, for orchestrating work on off-chain engines that must be
/// registered with the on-chain upon completion.
///
/// The general flow is:
///
/// 1. Task is initialized on-chain using [`TaskQueue::push_task`].
/// 2. The off-chain component notices the task using [`TaskQueue::first_if_unhandled`].
/// 3. The off-chain component solves the task, and triggers an notification to the on-chain using [`TaskQueue::report_completion`]
/// 4. The on-chain component receives the notification, calls [`TaskQueue::mark_completed_by_engine`], and
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
    queue: Vec<TaskId>,
    /// The identifier of the
    next_task_id: TaskId,
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
            queue: vec![],
            next_task_id: 1,
            tasks: AvlTreeMap::new(),
        }
    }

    /// Get the task id of the current task.
    pub fn _task_id_of_current(&self) -> TaskId {
        *self.queue.first().unwrap_or(&0)
    }

    /// Add another task to the task queue.
    ///
    /// Must be called on-chain.
    pub fn push_task(&mut self, definition: DefinitionT) -> TaskId {
        let task_id = self.next_task_id;
        self.tasks.insert(
            task_id,
            Task {
                id: task_id,
                definition,
                completion_data: vec![None; self.num_engines as usize],
            },
        );
        self.queue.push(task_id);
        self.next_task_id += 1;
        task_id
    }

    /// Get the task with the given id.
    pub fn get_task(&self, task_id: TaskId) -> Option<Task<DefinitionT, CompletionT>> {
        self.tasks.get(&task_id)
    }

    /// Get the current task if the off-chain haven't completed it.
    ///
    /// Must be called off-chain.
    pub fn _first_if_unhandled(
        &self,
        context: &mut OffChainContext,
    ) -> Option<Task<DefinitionT, CompletionT>> {
        self.queue
            .iter()
            .map(|&task_id| self.get_if_unhandled(context, task_id))
            .next()?
    }

    /// Get the next task that the off-chain haven't completed yet.
    /// Returns none if there are no tasks the off-chain hasn't completed.
    ///
    /// Must be called off-chain.
    pub fn next_unhandled(
        &self,
        context: &mut OffChainContext,
    ) -> Option<Task<DefinitionT, CompletionT>> {
        self.queue
            .iter()
            .filter_map(|&task_id| self.get_if_unhandled(context, task_id))
            .next()
    }
    /// Get the next [`limit`] tasks that the off-chain haven't completed yet.
    /// Returns fewer tasks if there are fewer tasks the off-chain hasn't completed.
    ///
    /// Must be called off-chain.
    pub fn next_multiple_unhandled(
        &self,
        context: &mut OffChainContext,
        limit: usize,
    ) -> Vec<Task<DefinitionT, CompletionT>> {
        self.queue
            .iter()
            .filter_map(|&task_id| self.get_if_unhandled(context, task_id))
            .take(limit)
            .collect()
    }

    /// Get a specific task if the off-chain haven't completed it.
    ///
    /// Must be called off-chain.
    pub fn get_if_unhandled(
        &self,
        context: &mut OffChainContext,
        task_id: TaskId,
    ) -> Option<Task<DefinitionT, CompletionT>> {
        let engine_finished_task = self
            .completion_status_storage(context)
            .get(&task_id)
            .is_some();

        if engine_finished_task {
            None
        } else {
            self.get_task(task_id)
        }
    }

    /// Remove the task with the given id.
    ///
    /// Must be called on-chain.
    pub fn _remove_task(&mut self, remove_task: TaskId) {
        self.queue.retain(|&i| i != remove_task);
        self.tasks.remove(&remove_task)
    }

    /// Removes all uncompleted tasks.
    ///
    /// Must be called on-chain.
    pub fn cancel_all_pending_tasks(&mut self) {
        self.queue.clear();
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
    /// - [`gas_for_cpu`]: The amount of gas to be sent for cpu usage. The gas required for network is automatically calculated.
    pub fn report_completion<RpcGeneratorT>(
        &self,
        context: &mut OffChainContext,
        task: Task<DefinitionT, CompletionT>,
        rpc_generator: RpcGeneratorT,
        completion: CompletionT,
        gas_for_cpu: u64,
    ) where
        RpcGeneratorT: FnOnce(TaskId, CompletionT) -> Vec<u8>,
    {
        context
            .call_contract(rpc_generator(task.id(), completion))
            .with_transport_fee_from_rpc()
            .with_additional_gas(gas_for_cpu)
            .send();

        self.completion_status_storage(context)
            .insert(task.id(), task.id());
    }

    /// Report the completion of multiple tasks to the on-chain smart-contract using a single transaction.
    ///
    /// Must be called off-chain.
    ///
    /// ## Arguments
    ///
    /// - [`context`]: Context used to send the transaction to the contract.
    /// - [`task_completions`]: The completed tasks and their completion data.
    /// - [`rpc_generator`]: Function used to create the RPC for a call to inform the on-chain of the completion.
    /// - [`gas_for_cpu`]: The amount of gas to be sent for cpu usage. The gas required for network is automatically calculated.
    pub fn report_completion_batched<RpcGeneratorT>(
        &self,
        context: &mut OffChainContext,
        rpc_generator: RpcGeneratorT,
        task_completions: Vec<(Task<DefinitionT, CompletionT>, CompletionT)>,
        gas_for_cpu: u64,
    ) where
        RpcGeneratorT: FnOnce(Vec<WithId<CompletionT>>) -> Vec<u8>,
    {
        let task_ids: Vec<TaskId> = task_completions.iter().map(|(task, _)| task.id).collect();
        let rpc = rpc_generator(
            task_completions
                .into_iter()
                .map(|(task, comp)| WithId::new(task.id, comp))
                .collect(),
        );
        context
            .call_contract(rpc)
            .with_transport_fee_from_rpc()
            .with_additional_gas(gas_for_cpu)
            .send();

        for task_id in task_ids {
            self.completion_status_storage(context)
                .insert(task_id, task_id);
        }
    }

    /// Marks the task as being completed by the given engine and with the given completion data.
    ///
    /// Must be called on-chain.
    pub fn mark_completed_by_engine(
        &mut self,
        engine_index: EngineIndex,
        task_id: TaskId,
        completion: CompletionT,
    ) {
        let mut task = self
            .tasks
            .get(&task_id)
            .unwrap_or_else(|| panic!("No task with given id {}", task_id));
        task.completion_data[engine_index as usize] = Some(completion);
        if task.is_complete() {
            self.queue.retain(|&i| i != task_id);
        }
        self.tasks.insert(task_id, task);
    }

    /// Manually mark that a task has finished.
    /// Used to complete a task if it e.g. only requires a number of engines to respond
    ///
    /// Must be called on-chain.
    pub fn mark_completion(&mut self, task_id: TaskId) {
        self.queue.retain(|&i| i != task_id);
    }

    /// Check if a task has been started, but not yet been completed.
    pub fn is_active(&self, task_id: TaskId) -> bool {
        self.queue.contains(&task_id)
    }

    /// Storage used to track the off-chain completion status of the task.
    ///
    /// Must be called off-chain.
    fn completion_status_storage(
        &self,
        context: &mut OffChainContext,
    ) -> OffChainStorage<'_, TaskId, TaskId> {
        context.storage(&self.bucket_id)
    }
}

/// Add an id to a value
#[derive(ReadWriteRPC, CreateTypeSpec)]
pub struct WithId<T> {
    /// Identifier
    pub id: TaskId,
    /// Value
    pub value: T,
}

impl<T> WithId<T> {
    /// Create a [`WithId`]
    pub fn new(id: TaskId, value: T) -> Self {
        Self { id, value }
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

        assert_eq!(queue._task_id_of_current(), 0);

        queue.push_task(Empty {});
        assert_eq!(queue._task_id_of_current(), 1);

        queue.mark_completed_by_engine(0, 1, Empty {});
        queue.mark_completed_by_engine(1, 1, Empty {});

        queue.push_task(Empty {});
        assert_eq!(queue._task_id_of_current(), 2);
        queue.mark_completed_by_engine(0, 2, Empty {});
        queue.mark_completed_by_engine(1, 2, Empty {});

        queue.push_task(Empty {});
        assert_eq!(queue._task_id_of_current(), 3);
        queue.mark_completed_by_engine(0, 3, Empty {});
        queue.mark_completed_by_engine(1, 3, Empty {});
        assert_eq!(queue._task_id_of_current(), 0);
    }

    /// Can push many times before beginning to complete tasks.
    #[test]
    fn test_queue_push_many_complete_many() {
        let mut queue: TaskQueue<Empty, Empty> = TaskQueue::new(vec![1, 2, 3], 2);

        assert_eq!(queue._task_id_of_current(), 0);

        queue.push_task(Empty {});
        queue.push_task(Empty {});
        queue.push_task(Empty {});

        assert_eq!(queue._task_id_of_current(), 1);

        queue.mark_completed_by_engine(0, 1, Empty {});
        queue.mark_completed_by_engine(1, 1, Empty {});

        assert_eq!(queue._task_id_of_current(), 2);
        queue.mark_completed_by_engine(0, 2, Empty {});
        queue.mark_completed_by_engine(1, 2, Empty {});

        assert_eq!(queue._task_id_of_current(), 3);
        queue.mark_completed_by_engine(0, 3, Empty {});
        queue.mark_completed_by_engine(1, 3, Empty {});
        assert_eq!(queue._task_id_of_current(), 0);
    }

    /// All completion data is available once all engines have been marked as completing the task.
    #[test]
    fn task_completion_data() {
        let mut queue: TaskQueue<Empty, Empty> = TaskQueue::new(vec![1, 2, 3], 2);

        assert_eq!(queue.get_task(1), None);

        queue.push_task(Empty {});

        assert_eq!(queue.get_task(1).unwrap().all_completion_data(), None);

        queue.mark_completed_by_engine(0, 1, Empty {});

        assert_eq!(queue.get_task(1).unwrap().all_completion_data(), None);

        queue.mark_completed_by_engine(1, 1, Empty {});

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
        queue._remove_task(1);
        assert_eq!(queue._task_id_of_current(), 0);

        queue.push_task(Empty {});
        queue._remove_task(2);
        assert_eq!(queue._task_id_of_current(), 0);

        queue.push_task(Empty {});
        queue._remove_task(3);
        assert_eq!(queue._task_id_of_current(), 0);

        queue.push_task(Empty {});
        assert!(queue.get_task(4).is_some());
        assert_eq!(queue._task_id_of_current(), 4);
    }
}

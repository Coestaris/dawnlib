use crate::registry::{AssetRegistry, AssetState};
use crate::requests::task::{AssetTaskID, TaskCommand};
use crate::requests::{AssetRequest, AssetRequestID};
use crate::AssetID;
use log::debug;
use std::collections::HashSet;
use thiserror::Error;

pub(crate) struct TaskPool {
    requests: Vec<Request>,
    peekable: bool,
}

#[derive(Clone, Debug)]
enum TaskState {
    Pending,
    Processing,
    Done,
}

#[derive(Debug, Clone)]
pub(crate) struct Task {
    pub id: AssetTaskID,
    pub command: TaskCommand,
    dependencies: HashSet<AssetTaskID>,
    state: TaskState,
}

struct Request {
    id: AssetRequestID,
    pending: Vec<Task>,
}

#[derive(Debug, Clone, Error)]
pub enum RequestError {
    #[error("Asset not found: {0}")]
    AssetNotFound(AssetID),
    #[error("Circular dependency detected for asset: {0}")]
    CircularDependency(AssetID),
    #[error("Request already exists for asset: {0}")]
    Other(String),
}

pub(crate) enum TaskDoneResult {
    Ok,
    RequestCompleted(AssetRequestID),
}

pub(crate) enum PeekResult {
    Peeked(Task),
    NoPendingTasks,
    UnwrapFailed(AssetTaskID, String),
}

impl TaskPool {
    pub fn new() -> Self {
        TaskPool {
            requests: Vec::new(),
            peekable: false,
        }
    }

    fn collect_load_tasks(
        qid: AssetRequestID,
        aid: AssetID,
        registry: &AssetRegistry,
    ) -> Result<Vec<Task>, RequestError> {
        let header = registry
            .get_header(&aid)
            .map_err(|_| RequestError::AssetNotFound(aid.clone()))?;
        let mut tasks = Vec::new();

        // Handle all dependencies of the asset
        for dep in header.dependencies.iter() {
            // TODO: Protect against circular dependencies
            tasks.extend(Self::collect_load_tasks(qid, dep.clone(), registry)?);
        }

        let state = registry
            .get_state(&aid)
            .map_err(|_| RequestError::AssetNotFound(aid.clone()))?;
        let dep_ids = tasks.iter().map(|task| task.id).collect::<HashSet<_>>();
        match state {
            AssetState::Empty => {
                // Load Asset to the RAM and then process it
                let load_id = AssetTaskID::new(qid.clone());
                tasks.push(Task {
                    id: load_id.clone(),
                    command: TaskCommand::Read(aid.clone()),
                    dependencies: dep_ids,
                    state: TaskState::Pending,
                });
                tasks.push(Task {
                    id: AssetTaskID::new(qid.clone()),
                    command: TaskCommand::Load(aid.clone()),
                    dependencies: HashSet::from([load_id]),
                    state: TaskState::Pending,
                });
            }
            AssetState::Read(_) => {
                // Asset is already loaded, just process it
                tasks.push(Task {
                    id: AssetTaskID::new(qid.clone()),
                    command: TaskCommand::Load(aid.clone()),
                    dependencies: dep_ids,
                    state: TaskState::Pending,
                });
            }
            AssetState::Loaded(_, _) => {
                // Asset is already loaded, no need to do anything
                // This is a no-op, but we can log it if needed
            }
        }

        Ok(tasks)
    }

    pub fn request(&mut self, request: AssetRequest) -> AssetRequestID {
        AssetRequestID::new()
    }

    pub fn peek_task(&mut self) -> PeekResult {
        if !self.peekable {
            return PeekResult::NoPendingTasks;
        }

        for request in self.requests.iter_mut() {
            // Find the pending task with empty dependencies
            if let Some(index) = request.pending.iter().position(|task| {
                matches!(task.state, TaskState::Pending) && task.dependencies.is_empty()
            }) {
                // Mark the task as processing
                let task = request.pending.get_mut(index).unwrap();
                task.state = TaskState::Processing;

                // Return the task
                debug!("Peeking task: {:?}", task);
                return PeekResult::Peeked(task.clone());
            }
        }

        debug!("No pending tasks found, resetting peekable flag");
        self.peekable = false;
        PeekResult::NoPendingTasks
    }

    pub fn task_done(&mut self, task_id: AssetTaskID) -> TaskDoneResult {
        // Mark the task as done and remove it from all the dependencies
        let qid = task_id.as_request();
        let request = self.requests.iter_mut().find(|q| q.id == qid).unwrap();
        let task_index = request
            .pending
            .iter()
            .position(|task| task.id == task_id)
            .unwrap();

        request.pending.get_mut(task_index).unwrap().state = TaskState::Done;
        for task in request.pending.iter_mut() {
            // Remove the task from dependencies of other tasks
            task.dependencies.remove(&task_id);
        }

        // If there are no more pending tasks, we can remove the request
        if request
            .pending
            .iter()
            .all(|task| matches!(task.state, TaskState::Done))
        {
            debug!("All tasks in request {} are done, removing request", qid);
            self.requests.retain(|q| q.id != qid);
            return TaskDoneResult::RequestCompleted(qid);
        }

        self.peekable = true;
        TaskDoneResult::Ok
    }

    pub fn task_failed(&mut self, task_id: AssetTaskID) {
        // Remove the request that contains the task
        let qid = task_id.as_request();
        self.requests.retain(|request| request.id == qid);
        self.peekable = true;
    }
}

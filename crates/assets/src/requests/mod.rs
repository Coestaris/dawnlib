use crate::registry::{AssetRegistry, AssetState};
use crate::AssetID;
use log::debug;
use std::collections::HashSet;
use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct AssetRequest(usize);

impl std::fmt::Display for AssetRequest {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "AssetRequest({})", self.0)
    }
}

impl Default for AssetRequest {
    fn default() -> Self {
        AssetRequest(0)
    }
}

impl AssetRequest {
    pub fn new() -> Self {
        static NEXT_ID: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
        let id = NEXT_ID.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        AssetRequest(id)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct AssetTaskID(AssetRequest, usize);

impl AssetTaskID {
    pub fn new(qid: AssetRequest) -> Self {
        static NEXT_ID: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
        let id = NEXT_ID.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        AssetTaskID(qid, id)
    }

    pub fn as_request(&self) -> AssetRequest {
        self.0
    }

    pub fn as_task_id(&self) -> usize {
        self.1
    }
}

impl std::fmt::Display for AssetTaskID {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "TaskID({}, {})", self.0, self.1)
    }
}

pub(crate) struct TaskPool {
    requests: Vec<Request>,
    peekable: bool,
}

#[derive(Debug, Clone)]
pub(crate) enum TaskCommand {
    IR(AssetID),
    Load(AssetID),
    Free(AssetID),
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
    id: AssetRequest,
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
    RequestCompleted(AssetRequest),
}

impl TaskPool {
    pub fn new() -> Self {
        TaskPool {
            requests: Vec::new(),
            peekable: false,
        }
    }

    fn collect_load_tasks(
        qid: AssetRequest,
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
                    command: TaskCommand::IR(aid.clone()),
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
            AssetState::IR(_) => {
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

    pub fn request_load(
        &mut self,
        aid: AssetID,
        registry: &AssetRegistry,
    ) -> Result<AssetRequest, RequestError> {
        let qid = AssetRequest::new();

        // Add the request to the pool
        self.requests.push(Request {
            id: qid,
            pending: Self::collect_load_tasks(qid.clone(), aid, registry)?,
        });

        // Return the request ID
        self.peekable = true;
        Ok(qid)
    }

    pub fn request_load_all(
        &mut self,
        registry: &AssetRegistry,
    ) -> Result<AssetRequest, RequestError> {
        let qid = AssetRequest::new();
        let mut tasks = Vec::new();
        for aid in registry.keys() {
            tasks.extend(Self::collect_load_tasks(qid, aid.clone(), registry)?);
        }

        let request = Request {
            id: qid,
            pending: tasks,
        };

        // Add the request to the pool
        self.requests.push(request);
        // Return the request ID
        self.peekable = true;
        Ok(qid)
    }

    pub fn peek_task(&mut self) -> Option<Task> {
        if !self.peekable {
            return None;
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
                return Some(task.clone());
            }
        }

        debug!("No pending tasks found, resetting peekable flag");
        self.peekable = false;
        None
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

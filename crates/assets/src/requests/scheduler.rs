use crate::registry::{AssetRegistry, AssetState, RegistryError};
use crate::requests::task::{AssetTaskID, TaskCommand};
use crate::requests::{AssetRequest, AssetRequestID, AssetRequestQuery};
use crate::AssetID;
use log::{debug, error, info};
use std::collections::HashSet;
use thiserror::Error;

#[derive(Clone, Debug, PartialEq, Eq)]
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

#[derive(Debug, Clone)]
enum RequestPromise {
    Pending(AssetRequestID, AssetRequest),
    Unwrapped(AssetRequestID, Vec<Task>),
}

pub(crate) struct Scheduler {
    promises: Vec<RequestPromise>,
    peekable: bool,
}

#[derive(Error, Debug)]
pub(crate) enum TaskFinishedError {
    #[error("Task finished for unknown request {0} (task {1})")]
    UnknownRequest(AssetTaskID, AssetRequestID),
    #[error("Task finished for unknown task {0} in request {1}")]
    UnknownTask(AssetTaskID, AssetRequestID),
    #[error("Task {0} failed for command {1:?}: {2}")]
    TaskFailed(AssetTaskID, Option<TaskCommand>, anyhow::Error),
    #[error("Finish of non-unwrapped request {0}")]
    NonUnwrappedRequest(AssetRequestID),
}

pub(crate) enum TaskDoneResult {
    Ok,
    RequestFinished(AssetRequestID, Result<(), TaskFinishedError>),
}

#[derive(Debug, Clone, Error)]
pub(crate) enum PeekError {
    #[error("Circular dependency detected for asset {0}")]
    CircularDependency(AssetID),
    #[error("Registry error: {0}")]
    RegistryError(#[from] RegistryError),
}

#[derive(Debug, Clone)]
pub(crate) enum PeekResult {
    Peeked(Task),
    EmptyUnwrap(AssetTaskID),
    NoPendingTasks,
    UnwrapFailed(AssetTaskID, PeekError),
}

impl Scheduler {
    pub fn new() -> Self {
        Scheduler {
            promises: Vec::new(),
            peekable: false,
        }
    }

    pub fn request(&mut self, request: AssetRequest) -> AssetRequestID {
        let rid = AssetRequestID::new();
        self.promises.push(RequestPromise::Pending(rid, request));
        self.peekable = true;
        rid
    }

    fn collect_tasks_for_asset(
        rid: AssetRequestID,
        aid: AssetID,
        registry: &AssetRegistry,
        deps: bool,
        stack: &mut Vec<AssetID>,
        constructor: &impl Fn(
            AssetRequestID,
            &AssetRegistry,
            AssetID,
            HashSet<AssetTaskID>,
        ) -> Result<Vec<Task>, PeekError>,
    ) -> Result<Vec<Task>, PeekError> {
        // Prevent circular dependencies.
        if stack.contains(&aid) {
            return Err(PeekError::CircularDependency(aid));
        }

        // If deps is true, we need to load dependencies first.
        let mut tasks = Vec::new();

        let mut dependencies = HashSet::new();
        if deps {
            stack.push(aid.clone());
            let header = registry.get_header(&aid)?;
            for dep in &header.dependencies {
                let deps = Self::collect_tasks_for_asset(
                    rid,
                    dep.clone(),
                    registry,
                    true,
                    stack,
                    constructor,
                )?;

                // Link the dependencies to the current task.
                for task_id in deps.iter().map(|t| t.id) {
                    dependencies.insert(task_id);
                }

                // Add dependencies to the task list.
                tasks.extend(deps);
            }
            stack.pop();
        }

        // Finally, add the task for the current asset.
        tasks.extend(constructor(rid, registry, aid.clone(), dependencies)?);
        Ok(tasks)
    }

    fn print_task_tree(tasks: &Vec<Task>, task: &Task, depth: usize) {
        println!(
            "{} Task ID: {}, Command: {:?}, State: {:?}",
            " ".repeat(depth),
            task.id,
            task.command,
            task.state
        );
        for dep in task.dependencies.iter() {
            if let Some(dep_task) = tasks.iter().find(|t| &t.id == dep) {
                Self::print_task_tree(tasks, &dep_task, depth + 2);
            } else {
                error!("Dependency task {:?} not found in task list", dep);
            }
        }
    }

    fn collect_tasks_for_query(
        rid: AssetRequestID,
        query: AssetRequestQuery,
        registry: &AssetRegistry,
        deps: bool,
        constructor: &impl Fn(
            AssetRequestID,
            &AssetRegistry,
            AssetID,
            HashSet<AssetTaskID>,
        ) -> Result<Vec<Task>, PeekError>,
    ) -> Result<Vec<Task>, PeekError> {
        let ids = match query {
            AssetRequestQuery::ByID(id) => vec![id],
            AssetRequestQuery::ByTag(tag) => registry
                .keys()
                .filter(|id| {
                    if let Ok(header) = registry.get_header(id) {
                        header.tags.contains(&tag)
                    } else {
                        false
                    }
                })
                .cloned()
                .collect(),
            AssetRequestQuery::ByTags(tags) => registry
                .keys()
                .filter(|id| {
                    if let Ok(header) = registry.get_header(id) {
                        tags.iter().all(|tag| header.tags.contains(tag))
                    } else {
                        false
                    }
                })
                .cloned()
                .collect(),
            AssetRequestQuery::All => registry.keys().cloned().collect(),
            AssetRequestQuery::ByType(asset_type) => registry
                .keys()
                .filter(|id| {
                    if let Ok(header) = registry.get_header(id) {
                        header.asset_type == asset_type
                    } else {
                        false
                    }
                })
                .cloned()
                .collect(),
        };

        let mut all_tasks = Vec::new();
        for id in ids {
            let mut stack = Vec::new();
            let tasks = Self::collect_tasks_for_asset(
                rid,
                id.clone(),
                registry,
                deps,
                &mut stack,
                constructor,
            )?;
            all_tasks.extend(tasks);
        }

        // Merge tasks with the same AssetID and command, joining their dependencies.
        // If any other task is dependent on the merged task, we should replace it with the merged task.
        let mut task_map = std::collections::HashMap::new();
        for task in &all_tasks {
            task_map
                .entry(task.command.clone())
                .and_modify(|existing_task: &mut Task| {
                    debug!("Merging: {:?} and {:?}", existing_task, task);
                    existing_task.dependencies.extend(task.dependencies.clone());
                })
                .or_insert(task.clone());
        }
        // Update the task dependencies to point to the merged task.
        let mut merged_map = task_map.clone();
        for task in merged_map.values_mut() {
            let mut new_dependencies = HashSet::new();
            for dependency in &task.dependencies {
                // Find the merged task for the dependency.
                let old_command = all_tasks
                    .iter()
                    .find(|t| t.id == dependency.clone())
                    .unwrap();
                // Replace the dependency with the merged task.
                new_dependencies.insert(task_map.get(&old_command.command).unwrap().id.clone());
            }
            task.dependencies = new_dependencies;
        }

        // let all_tasks= all_tasks.into_iter().collect();
        // for task in merged_map.values() {
        //     Self::print_task_tree(&all_tasks, &task, 0);
        // }

        Ok(merged_map.values().cloned().collect())
    }

    fn read_constructor(
        rid: AssetRequestID,
        registry: &AssetRegistry,
        aid: AssetID,
        dependencies: HashSet<AssetTaskID>,
    ) -> Result<Vec<Task>, PeekError> {
        // If the asset is already read or loaded, no need to read it again.
        match registry.get_state(&aid)? {
            AssetState::Empty => Ok(vec![Task {
                id: AssetTaskID::new(rid),
                command: TaskCommand::Read(aid),
                dependencies,
                state: TaskState::Pending,
            }]),
            _ => Ok(vec![]),
        }
    }

    fn load_constructor(
        rid: AssetRequestID,
        registry: &AssetRegistry,
        aid: AssetID,
        dependencies: HashSet<AssetTaskID>,
    ) -> Result<Vec<Task>, PeekError> {
        // If the asset is already loaded, no need to load it again.
        match registry.get_state(&aid)? {
            AssetState::Empty => {
                let load_tid = AssetTaskID::new(rid);
                Ok(vec![
                    Task {
                        id: load_tid.clone(),
                        command: TaskCommand::Read(aid.clone()),
                        dependencies,
                        state: TaskState::Pending,
                    },
                    Task {
                        id: AssetTaskID::new(rid),
                        command: TaskCommand::Load(aid),
                        dependencies: vec![load_tid].into_iter().collect(),
                        state: TaskState::Pending,
                    },
                ])
            }
            AssetState::Loaded(_, _) => Ok(vec![Task {
                id: AssetTaskID::new(rid),
                command: TaskCommand::Read(aid),
                dependencies,
                state: TaskState::Pending,
            }]),
            _ => Ok(vec![]),
        }
    }

    fn load_constructor_no_dep(
        rid: AssetRequestID,
        registry: &AssetRegistry,
        aid: AssetID,
        _dependencies: HashSet<AssetTaskID>,
    ) -> Result<Vec<Task>, PeekError> {
        // If the asset is already loaded, no need to load it again.
        match registry.get_state(&aid)? {
            AssetState::Loaded(_, _) => Ok(vec![Task {
                id: AssetTaskID::new(rid),
                command: TaskCommand::Read(aid),
                dependencies: HashSet::new(),
                state: TaskState::Pending,
            }]),
            _ => Ok(vec![]),
        }
    }

    fn free_constructor(
        rid: AssetRequestID,
        registry: &AssetRegistry,
        aid: AssetID,
        dependencies: HashSet<AssetTaskID>,
    ) -> Result<Vec<Task>, PeekError> {
        match registry.get_state(&aid)? {
            AssetState::Loaded(_, _) => Ok(vec![Task {
                id: AssetTaskID::new(rid),
                command: TaskCommand::Free(aid),
                dependencies,
                state: TaskState::Pending,
            }]),
            _ => Ok(vec![]),
        }
    }

    fn unwrap(
        rid: AssetRequestID,
        request: AssetRequest,
        registry: &AssetRegistry,
    ) -> Result<Vec<Task>, PeekError> {
        match request {
            AssetRequest::Enumerate => Ok(vec![Task {
                id: AssetTaskID::new(rid),
                command: TaskCommand::Enumerate,
                dependencies: HashSet::new(),
                state: TaskState::Pending,
            }]),
            AssetRequest::Read(query) => {
                Self::collect_tasks_for_query(rid, query, registry, true, &Self::read_constructor)
            }
            AssetRequest::ReadNoDeps(query) => {
                Self::collect_tasks_for_query(rid, query, registry, false, &Self::read_constructor)
            }
            AssetRequest::Load(query) => {
                Self::collect_tasks_for_query(rid, query, registry, true, &Self::load_constructor)
            }
            AssetRequest::LoadNoDeps(query) => Self::collect_tasks_for_query(
                rid,
                query,
                registry,
                false,
                &Self::load_constructor_no_dep,
            ),
            AssetRequest::Free(query) => {
                Self::collect_tasks_for_query(rid, query, registry, true, &Self::free_constructor)
            }
            AssetRequest::FreeNoDeps(query) => {
                Self::collect_tasks_for_query(rid, query, registry, false, &Self::free_constructor)
            }
        }
    }

    pub fn peek(&mut self, registry: &AssetRegistry) -> PeekResult {
        if !self.peekable {
            return PeekResult::NoPendingTasks;
        }

        // Processing the first promise. If it is still pending, we need to unwrap it into tasks.
        let to_unwrap = match self.promises.first() {
            None => {
                debug!("No pending requests");
                self.peekable = false;
                return PeekResult::NoPendingTasks;
            }
            Some(RequestPromise::Pending(rid, request)) => Some((rid.clone(), request.clone())),
            _ => None,
        };
        if let Some((rid, request)) = to_unwrap {
            match Self::unwrap(rid.clone(), request.clone(), registry) {
                Ok(tasks) if !tasks.is_empty() => {
                    // Replace the pending promise with the unwrapped tasks.
                    self.promises.remove(0);
                    self.promises
                        .insert(0, RequestPromise::Unwrapped(rid.clone(), tasks.clone()));
                    debug!("Unwrapped request {} ({:?}) into {:?}", rid, request, tasks);
                }
                Ok(_) => return PeekResult::EmptyUnwrap(AssetTaskID::new(rid.clone())),
                Err(e) => {
                    // The request will be removed in the next call to task_finished.
                    // Hoping upper layers will handle the error appropriately.
                    return PeekResult::UnwrapFailed(AssetTaskID::new(rid.clone()), e);
                }
            };
        }

        // Select any task that is pending and has no dependencies.
        let (rid, tasks) = match &mut self.promises[0] {
            RequestPromise::Unwrapped(rid, tasks) => (rid.clone(), tasks),
            _ => unreachable!(),
        };
        if let Some(pos) = tasks
            .iter()
            .position(|t| t.state == TaskState::Pending && t.dependencies.is_empty())
        {
            // Mark the task as processing and return it.
            let task = &mut tasks[pos];
            task.state = TaskState::Processing;

            debug!("Peeking task {} from request {}", task.id, rid);
            PeekResult::Peeked(task.clone())
        } else {
            // No pending tasks available right now.
            // Hoping some will appear after some tasks finish.
            self.peekable = false;
            PeekResult::NoPendingTasks
        }
    }

    pub fn task_finished(
        &mut self,
        task_id: AssetTaskID,
        result: anyhow::Result<()>,
    ) -> TaskDoneResult {
        let rid = task_id.as_request();
        self.peekable = true;

        // Find the request index.
        let request_index = match self.promises.iter().position(|p| match p {
            RequestPromise::Pending(other, _) | RequestPromise::Unwrapped(other, _) => {
                rid.0 == other.0
            }
        }) {
            Some(index) => index,
            None => {
                return TaskDoneResult::RequestFinished(
                    rid,
                    Err(TaskFinishedError::UnknownRequest(task_id, rid)),
                );
            }
        };

        let mut command = None;
        if !result.is_err() {
            // Mark the task done and remove it from dependencies of other tasks.
            match &mut self.promises[request_index] {
                RequestPromise::Unwrapped(_, tasks) => {
                    let task_index = match tasks.iter().position(|t| t.id == task_id) {
                        Some(index) => index,
                        None => {
                            return TaskDoneResult::RequestFinished(
                                rid,
                                Err(TaskFinishedError::UnknownTask(task_id, rid)),
                            );
                        }
                    };

                    tasks[task_index].state = TaskState::Done;
                    command = Some(tasks[task_index].command.clone());
                    let completed_task_id = tasks[task_index].id;
                    for task in tasks.iter_mut() {
                        task.dependencies.remove(&completed_task_id);
                    }

                    if tasks.iter().any(|t| t.state != TaskState::Done) {
                        // There's still some tasks pending, return Ok.
                        return TaskDoneResult::Ok;
                    }
                }
                _ => {
                    // This should never happen,
                    // as we unwrap requests before processing tasks.
                }
            };
        } else {
            // If the task failed, find the command for error reporting.
            match &mut self.promises[request_index] {
                RequestPromise::Unwrapped(_, tasks) => {
                    match tasks.iter().position(|t| t.id == task_id) {
                        Some(index) => {
                            command = Some(tasks[index].command.clone());
                        }
                        None => {}
                    }
                }
                _ => {}
            }
        }

        let result = result.map_err(|e| TaskFinishedError::TaskFailed(task_id, command, e));

        // If all tasks are done or failed, remove the request and return RequestFinished.
        self.promises.remove(request_index);
        TaskDoneResult::RequestFinished(rid, result)
    }
}

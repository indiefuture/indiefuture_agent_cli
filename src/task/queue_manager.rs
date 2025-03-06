use std::sync::{Arc, Mutex, Once};
use crate::task::{SubTask, SubTaskQueue};
use crate::error::AgentResult;

static QUEUE_MANAGER_INIT: Once = Once::new();
static mut QUEUE_MANAGER: Option<Arc<SubTaskQueueManager>> = None;

/// Manages the global subtask queue
pub struct SubTaskQueueManager {
    queue: SubTaskQueue,
}

impl SubTaskQueueManager {
    /// Get the global instance of the queue manager
    pub fn global() -> Arc<Self> {
        unsafe {
            QUEUE_MANAGER_INIT.call_once(|| {
                QUEUE_MANAGER = Some(Arc::new(SubTaskQueueManager {
                    queue: SubTaskQueue::new(),
                }));
            });
            
            QUEUE_MANAGER.clone().unwrap()
        }
    }
    
    /// Add a subtask to the queue
    pub fn add_queued_subtask(&self, subtask: SubTask) {
        self.queue.add(subtask);
    }
    
    /// Get the next subtask from the queue
    pub fn next_subtask(&self) -> Option<SubTask> {
        self.queue.next()
    }
    
    /// Check if the queue is empty
    pub fn is_queue_empty(&self) -> bool {
        self.queue.is_empty()
    }
    
    /// Get the current queue length
    pub fn queue_length(&self) -> usize {
        self.queue.len()
    }
    
    /// List all subtasks in the queue
    pub fn list_subtasks(&self) -> Vec<SubTask> {
        self.queue.list()
    }
}
//! Async job queue for long-running executions

use std::collections::HashMap;
use std::sync::Arc;

use chrono::{DateTime, Utc};
use tokio::sync::RwLock;
use uuid::Uuid;

use crate::models::{ExecuteRequest, ExecuteResponse, JobStatus, JobStatusResponse};

/// Job information stored in queue
#[derive(Debug, Clone)]
pub struct Job {
    pub id: Uuid,
    pub user_id: String,
    pub request: ExecuteRequest,
    pub status: JobStatus,
    pub result: Option<ExecuteResponse>,
    pub queued_at: DateTime<Utc>,
    pub started_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
}

/// In-memory job queue
pub struct JobQueue {
    jobs: Arc<RwLock<HashMap<Uuid, Job>>>,
}

impl JobQueue {
    pub fn new() -> Self {
        Self {
            jobs: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Submit a new job
    pub async fn submit(&self, user_id: String, request: ExecuteRequest) -> Uuid {
        let job_id = Uuid::new_v4();
        let job = Job {
            id: job_id,
            user_id,
            request,
            status: JobStatus::Queued,
            result: None,
            queued_at: Utc::now(),
            started_at: None,
            completed_at: None,
        };

        self.jobs.write().await.insert(job_id, job);
        job_id
    }

    /// Get job status
    pub async fn get_status(&self, job_id: &Uuid) -> Option<JobStatusResponse> {
        let jobs = self.jobs.read().await;
        let job = jobs.get(job_id)?;

        Some(JobStatusResponse {
            job_id: job.id,
            status: job.status,
            result: job.result.clone(),
            queued_at: job.queued_at,
            started_at: job.started_at,
            completed_at: job.completed_at,
        })
    }

    /// Update job status
    pub async fn update_status(&self, job_id: &Uuid, status: JobStatus) {
        if let Some(job) = self.jobs.write().await.get_mut(job_id) {
            job.status = status;
            if status == JobStatus::Running && job.started_at.is_none() {
                job.started_at = Some(Utc::now());
            }
        }
    }

    /// Complete a job with result
    pub async fn complete(&self, job_id: &Uuid, result: ExecuteResponse) {
        if let Some(job) = self.jobs.write().await.get_mut(job_id) {
            job.status = match result.status {
                crate::models::ExecutionStatus::Success => JobStatus::Completed,
                crate::models::ExecutionStatus::Timeout => JobStatus::Timeout,
                _ => JobStatus::Failed,
            };
            job.result = Some(result);
            job.completed_at = Some(Utc::now());
        }
    }

    /// Get next queued job
    pub async fn get_next_queued(&self) -> Option<(Uuid, String, ExecuteRequest)> {
        let mut jobs = self.jobs.write().await;

        for (id, job) in jobs.iter_mut() {
            if job.status == JobStatus::Queued {
                job.status = JobStatus::Running;
                job.started_at = Some(Utc::now());
                return Some((*id, job.user_id.clone(), job.request.clone()));
            }
        }
        None
    }

    /// Clean up old completed jobs (older than 1 hour)
    pub async fn cleanup_old_jobs(&self) {
        let cutoff = Utc::now() - chrono::Duration::hours(1);
        self.jobs.write().await.retain(|_, job| {
            if let Some(completed_at) = job.completed_at {
                completed_at > cutoff
            } else {
                true // Keep queued/running jobs
            }
        });
    }
}

impl Default for JobQueue {
    fn default() -> Self {
        Self::new()
    }
}

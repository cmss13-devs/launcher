use std::sync::OnceLock;
use win32job::Job;

use crate::error::{CommandError, CommandResult};

static JOB: OnceLock<Job> = OnceLock::new();

pub fn init_job_object() -> CommandResult<()> {
    if JOB.get().is_some() {
        return Ok(());
    }

    let job =
        Job::create().map_err(|e| CommandError::Internal(format!("Failed to create job object: {e}")))?;

    let mut info = job
        .query_extended_limit_info()
        .map_err(|e| CommandError::Internal(format!("Failed to query job info: {e}")))?;

    info.limit_kill_on_job_close();

    job.set_extended_limit_info(&mut info)
        .map_err(|e| CommandError::Internal(format!("Failed to set job info: {e}")))?;

    job.assign_current_process().map_err(|e| {
        CommandError::Internal(format!("Failed to assign current process to job: {e}"))
    })?;

    tracing::info!("Job object initialized - child processes will terminate on launcher exit");

    // Use get_or_init to store the job. If another thread raced us, our job gets dropped.
    JOB.get_or_init(|| job);

    Ok(())
}

use std::sync::{Mutex, Arc, TryLockError};

struct Inner<T> {
    data: Option<T>
}

pub struct JobFuture<T> {
    value: Arc<Mutex<Inner<T>>>
}

impl<T> JobFuture<T> {
    /// Wait for a job to finish execution, and fetch the held value.
    /// ```
    /// # use gk_types_rs::job_system::thread::JobThread;
    /// # use gk_types_rs::job_system::future::JobFuture;
    /// let mut job_thread = JobThread::new();
    /// let future = job_thread.queue_job(|| 10);
    /// // Job will execute here
    /// job_thread.execute();
    /// let num = future.wait();
    /// assert_eq!(num, 10);
    /// ```
    pub fn wait(&self) -> T {
        loop {
            match self.value.try_lock() {
                Ok(mut inner) => {
                    if inner.data.is_some() {
                        return (*inner).data.take().unwrap();
                    }
                },
                Err(e) => {
                    if let TryLockError::Poisoned(e) = e {
                        panic!("couldn't take job future: {}", e);
                    }
                }
            }
            std::thread::yield_now();
        }
    }
}


pub(crate) struct WithinJobFuture<T> {
    value: Arc<Mutex<Inner<T>>>,
}

impl<T> WithinJobFuture<T> {
    pub(crate) fn new() -> (JobFuture<T>, WithinJobFuture<T>) {
        let wait_job_future = JobFuture {
            value: Arc::new(Mutex::new(Inner { data: None }))};

        let within_job_future = WithinJobFuture {
            value: wait_job_future.value.clone(),
        };

        return (wait_job_future, within_job_future);
    }

    pub(crate) fn set(&self, data: T) {
        let mut inner = self.value.lock().unwrap();
        (*inner).data = Some(data);
    }
}
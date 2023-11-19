use super::{system::QUEUE_CAPACITY, job_container::JobContainer, ring_queue::JobRingQueue};

pub(crate) struct ActiveJobs {
    work: Box<[JobContainer]>,
    count: usize
}

impl ActiveJobs {
    pub(crate) fn new() -> Self {
        let mut v: Vec<JobContainer> = Vec::with_capacity(QUEUE_CAPACITY);
        for _ in 0..QUEUE_CAPACITY {
            v.push(JobContainer::default());
        }
        return ActiveJobs { 
            work: v.into_boxed_slice(),
            count: 0
        }
    }

    pub(crate) fn collect_jobs(&mut self, queue: &mut JobRingQueue) {
        debug_assert!((self.count + queue.length) <= QUEUE_CAPACITY, "Too many job to be worked on");
        // All jobs in the unused part of the work array should have no bind.
        unsafe { 
            let work_ptr = self.work.as_mut_ptr();
            let queue_buf_ptr = queue.buffer.as_mut_ptr();
            std::ptr::swap_nonoverlapping(work_ptr.offset(self.count as isize), queue_buf_ptr, queue.length);
            // From the behaviour of invoke_all_jobs() replacing self's active work buffer with Job::default(), it can be assumed that swap will correct change the queue to hold default
            //std::ptr::write(&mut queue.buffer as *mut Job, Job::default());           
        }
        self.count += queue.length;
        queue.length = 0;
        queue.read_index = 0;
        queue.write_index = 0;
    }

    pub(crate) fn invoke_all_jobs(&mut self) {
        for i in 0..self.count {
            self.work[i].invoke();
            //let job = std::mem::take(&mut self.work[i]);
            //job.invoke();
            // is dropped
        }
        self.count = 0;
    }
}
use super::{job_container::JobContainer, system::QUEUE_CAPACITY};

pub(crate) struct JobRingQueue {
    pub(crate) buffer: Box<[JobContainer]>,
    pub(crate) length: usize,
    pub(crate) read_index: usize,
    pub(crate) write_index: usize
}

impl JobRingQueue {
    pub(crate) fn new() -> Self {
        let mut v: Vec<JobContainer> = Vec::with_capacity(QUEUE_CAPACITY);
        for _ in 0..QUEUE_CAPACITY {
            v.push(JobContainer::default());
        }
        return JobRingQueue { 
            buffer: v.into_boxed_slice(), 
            length: 0, 
            read_index: 0, 
            write_index: 0 
        }
    }

    pub(crate) fn is_full(&self) -> bool {
        return self.length == QUEUE_CAPACITY;
    }

    // pub(crate) fn is_empty(&self) -> bool {
    //     return self.length == 0;
    // }

    pub(crate) fn push(&mut self, mut job: JobContainer) {
        assert!(!self.is_full(), "Job ring queue is full");

        std::mem::swap(&mut self.buffer[self.write_index], &mut job);
        self.write_index = (self.write_index + 1) % QUEUE_CAPACITY;
        self.length += 1;
    }

    // pub(crate) fn pop(&mut self) -> Job {
    //     assert!(!self.is_empty(), "Job ring queue is empty");

    //     let out_job = std::mem::take(&mut self.buffer[self.read_index]);
    //     self.read_index = (self.read_index + 1) % QUEUE_CAPACITY;
    //     self.length -= 1;
    //     return out_job;
    // }
}
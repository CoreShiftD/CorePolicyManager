use crate::core::{IoHandle, JobHandle, JobRuntime, JobState, ProcessHandle};
use crate::arena::Arena;
use std::collections::HashMap;

const K: u64 = 0x9E3779B97F4A7C15;
const PROCESS_COUNT_KEY: u64 = 0xA1;
const IO_COUNT_KEY: u64 = 0xB2;

#[inline]
pub fn mix(a: u64, b: u64) -> u64 {
    a.wrapping_mul(K) ^ b
}

#[inline]
pub fn hash_job(job: &JobState) -> u64 {
    let mut h = 0;

    h ^= mix(1, job.id);
    h ^= mix(2, job.owner as u64);
    h ^= mix(3, job.lifecycle as u64);
    h ^= mix(4, job.io_state as u64);

    if job.timed_out {
        h ^= mix(5, 1);
    }

    if let Some(p) = job.process {
        h ^= mix(6, p.index as u64);
    }

    if let Some(io) = job.io {
        h ^= mix(7, io.index as u64);
    }

    h
}

pub struct CoreState {
    pub jobs: Arena<JobState>,
    pub(crate) job_id_map: HashMap<u64, JobHandle>,
    pub runtime: Vec<Option<JobRuntime>>,
    pub process_index: Vec<Option<JobHandle>>,
    pub io_index: Vec<Option<JobHandle>>,
    pub process_count: usize,
    pub io_count: usize,
    pub hash: u64,
}

impl Default for CoreState {
    fn default() -> Self {
        Self::new()
    }
}

impl CoreState {
    pub fn new() -> Self {
        Self {
            jobs: Arena::new(),
            job_id_map: HashMap::new(),
            runtime: Vec::new(),
            process_index: Vec::new(),
            io_index: Vec::new(),
            process_count: 0,
            io_count: 0,
            hash: 0,
        }
    }

    #[inline]
    pub fn job_handle(&self, id: u64) -> Option<JobHandle> {
        self.job_id_map.get(&id).copied()
    }

    #[inline]
    pub fn job(&self, h: JobHandle) -> &JobState {
        let job = self.jobs.get(h.index, h.generation);
        debug_assert!(job.is_some(), "Job handle dangling in jobs arena");
        job.unwrap()
    }

    #[inline]
    pub fn job_mut(&mut self, h: JobHandle) -> &mut JobState {
        let job = self.jobs.get_mut(h.index, h.generation);
        debug_assert!(job.is_some(), "Job handle dangling in jobs arena (mut)");
        job.unwrap()
    }

    #[inline]
    pub fn runtime(&self, h: JobHandle) -> &JobRuntime {
        debug_assert!((h.index as usize) < self.runtime.len(), "Runtime missing for job");
        self.runtime[h.index as usize].as_ref().unwrap()
    }

    #[inline]
    pub fn runtime_mut(&mut self, h: JobHandle) -> &mut JobRuntime {
        debug_assert!((h.index as usize) < self.runtime.len(), "Runtime missing for job (mut)");
        self.runtime[h.index as usize].as_mut().unwrap()
    }

    #[inline]
    pub fn job_by_process(&self, p: ProcessHandle) -> Option<JobHandle> {
        if (p.index as usize) < self.process_index.len() {
            self.process_index[p.index as usize]
        } else {
            None
        }
    }

    #[inline]
    pub fn job_by_io(&self, io: IoHandle) -> Option<JobHandle> {
        if (io.index as usize) < self.io_index.len() {
            self.io_index[io.index as usize]
        } else {
            None
        }
    }


    #[inline]
    pub fn remove_job(&mut self, id: u64) -> Option<JobState> {
        if let Some(h) = self.job_id_map.remove(&id) {
            if let Some(job) = self.jobs.remove(h.index, h.generation) {
                // XOR out job state from hash
                self.hash ^= mix(id, hash_job(&job));

                if let Some(p) = job.process {
                    self.remove_process_index(p);
                }
                if let Some(io) = job.io {
                    self.remove_io_index(io);
                }

                debug_assert!(self.runtime[h.index as usize].is_some(), "Runtime missing during cleanup");
                self.runtime[h.index as usize].take();

                return Some(job);
            }
        }
        None
    }

    #[inline]
    pub fn insert_job(&mut self, id: u64, owner: u32, exec: crate::core::ExecSpec, policy: crate::core::ExecPolicy) {
        if self.job_id_map.contains_key(&id) { return; }

        let (index, generation) = self.jobs.insert(JobState {
            id, owner, exec, policy, process: None, io: None, timed_out: false,
            lifecycle: crate::core::JobLifecycle::Submitted, io_state: crate::core::JobIoState::Pending,
        });

        let handle = crate::core::JobHandle {
            index, generation, _marker: std::marker::PhantomData,
        };

        self.job_id_map.insert(id, handle);

        if self.runtime.len() <= index as usize {
            self.runtime.resize(index as usize + 1, None);
        }
        self.runtime[index as usize] = Some(JobRuntime { process: None, io: None });

        let job = self.jobs.get(index, generation).unwrap();
        self.hash ^= mix(id, hash_job(job));
    }
    // Additional helpers for indexing mutations
    #[inline]
    pub fn remove_process_index(&mut self, p: ProcessHandle) {
        if (p.index as usize) < self.process_index.len() {
            if let Some(h) = self.process_index[p.index as usize] {
                // XOR out old value
                self.hash ^= mix(p.index as u64, h.index as u64);

                // Update count and XOR its change
                self.hash ^= mix(PROCESS_COUNT_KEY, self.process_count as u64);
                self.process_count -= 1;
                self.hash ^= mix(PROCESS_COUNT_KEY, self.process_count as u64);

                self.process_index[p.index as usize] = None;
            }
        }
    }

    #[inline]
    pub fn insert_process_index(&mut self, p: ProcessHandle, h: JobHandle) {
        if self.process_index.len() <= p.index as usize {
            self.process_index.resize(p.index as usize + 1, None);
        }

        // If there's an existing handle, XOR it out first
        if let Some(old) = self.process_index[p.index as usize] {
            self.hash ^= mix(p.index as u64, old.index as u64);
        } else {
            // Updating count since it was None
            self.hash ^= mix(PROCESS_COUNT_KEY, self.process_count as u64);
            self.process_count += 1;
            self.hash ^= mix(PROCESS_COUNT_KEY, self.process_count as u64);
        }

        self.process_index[p.index as usize] = Some(h);

        // XOR in new value
        self.hash ^= mix(p.index as u64, h.index as u64);
    }

    #[inline]
    pub fn remove_io_index(&mut self, io: IoHandle) {
        if (io.index as usize) < self.io_index.len() {
            if let Some(h) = self.io_index[io.index as usize] {
                // XOR out old value
                self.hash ^= mix(io.index as u64, h.index as u64);

                // Update count and XOR its change
                self.hash ^= mix(IO_COUNT_KEY, self.io_count as u64);
                self.io_count -= 1;
                self.hash ^= mix(IO_COUNT_KEY, self.io_count as u64);

                self.io_index[io.index as usize] = None;
            }
        }
    }

    #[inline]
    pub fn insert_io_index(&mut self, io: IoHandle, h: JobHandle) {
        if self.io_index.len() <= io.index as usize {
            self.io_index.resize(io.index as usize + 1, None);
        }

        // If there's an existing handle, XOR it out first
        if let Some(old) = self.io_index[io.index as usize] {
            self.hash ^= mix(io.index as u64, old.index as u64);
        } else {
            // Updating count since it was None
            self.hash ^= mix(IO_COUNT_KEY, self.io_count as u64);
            self.io_count += 1;
            self.hash ^= mix(IO_COUNT_KEY, self.io_count as u64);
        }

        self.io_index[io.index as usize] = Some(h);

        // XOR in new value
        self.hash ^= mix(io.index as u64, h.index as u64);
    }
}
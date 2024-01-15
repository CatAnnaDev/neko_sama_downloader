use std::{error::Error, sync::Arc, thread};
use crossbeam::queue::ArrayQueue;
use crate::{mod_file::cmd_line_parser::Args, warn};

pub fn max_thread_check(new_args: &Args) -> Result<usize, Box<dyn Error>> {
    let mut thread = new_args.thread as usize;
    let max_thread = thread::available_parallelism()?.get() * 4;
    if thread > max_thread {
        warn!("Max thread for your cpu is between 1 and {}", max_thread);
        thread = max_thread;
        warn!("Update thread for {} continue", thread);
    }
    Ok(thread)
}

pub struct ThreadPool {
    workers: Vec<Worker>,
    queue: Arc<ArrayQueue<Job>>,
}

pub enum Job {
    Task(Box<dyn FnOnce() + Send + 'static>),
    Terminate,
}

impl ThreadPool {
    pub fn new(mut size: usize, capa: usize) -> ThreadPool {
        if size <= 0 { size = 1 }
        let queue = Arc::new(ArrayQueue::<Job>::new(capa));
        let mut workers = Vec::with_capacity(size);
        for _ in 0..size {
            workers.push(Worker::new(Arc::clone(&queue)));
        }
        ThreadPool { workers, queue }
    }

    pub fn execute<F>(&mut self, f: F)
        where
            F: FnOnce() + Send + 'static,
    {
        let job = Job::Task(Box::new(f));
        let _ = &self.queue.push(job);

        for x in &mut self.workers {
            if let Some(a) = &x.thread {
                a.thread().unpark()
            }
        }
    }
}

impl Drop for ThreadPool {
    fn drop(&mut self) {
        for worker in &mut self.workers {
            let _ = self.queue.push(Job::Terminate);
            if let Some(thread) = worker.thread.take() {
                thread.thread().unpark();
            }
        }

        for worker in &mut self.workers {
            if let Some(thread) = worker.thread.take() {
                thread.thread().unpark();
                thread.join().unwrap();
            }
        }
    }
}

struct Worker {
    thread: Option<thread::JoinHandle<()>>,
}

impl Worker {
    fn new(queue: Arc<ArrayQueue<Job>>) -> Worker {
        let thread = thread::spawn(move || loop {
            match queue.pop() {
                Some(Job::Task(job)) => job(),
                Some(Job::Terminate) => break,
                None => thread::park(),
            }
        });

        Worker {
            thread: Some(thread),
        }
    }
}

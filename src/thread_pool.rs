use std::sync::Arc;
use crossbeam::queue::ArrayQueue;
use std::thread;

pub struct ThreadPool {
    workers: Vec<Worker>,
    queue: Arc<ArrayQueue<Job>>,
}

pub enum Job {
    Task(Box<dyn FnOnce() + Send + 'static>),
    Terminate,
}

impl ThreadPool {
    pub fn new(size: usize, capa: usize) -> ThreadPool {
        assert!(size > 0);

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
            if let Some(a) = &x.thread{
                a.thread().unpark()
            }
        }
    }
}

impl Drop for ThreadPool {
    fn drop(&mut self) {
        for _ in &self.workers {
            let _ = self.queue.push(Job::Terminate);
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
                None => thread::park()
            }
        });

        Worker {
            thread: Some(thread),
        }
    }
}
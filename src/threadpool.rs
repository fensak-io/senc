// Copyright (c) Fensak, LLC.
// SPDX-License-Identifier: MPL-2.0

use std::collections::HashSet;
use std::sync::{atomic, mpsc, Arc, Mutex};
use std::thread;
use std::time;

use anyhow::{anyhow, Result};
use log::*;
use uuid::Uuid;

use crate::engine;

// A thread pool with a set number of threads to run tasks on.
pub struct ThreadPool {
    workers: Vec<Worker>,
    tasks: HashSet<Uuid>,
    task_sender: Option<mpsc::Sender<Task>>,
    task_receiver: Arc<Mutex<mpsc::Receiver<Task>>>,
    result_receiver: mpsc::Receiver<Uuid>,
    has_quit: Arc<atomic::AtomicBool>,
}

impl ThreadPool {
    // Create a new ThreadPool.
    //
    // |size| is the number of threads in the pool. If set to 0, the size will be the number of
    // cores on the machine.
    //
    // |has_quit| is an Arc<AtomicBool> indicating whether the process has quit and is used to
    // gracefully shutdown the generation routine.
    pub fn new(
        context: engine::Context,
        size: usize,
        has_quit: Arc<atomic::AtomicBool>,
    ) -> ThreadPool {
        let thread_count = if size == 0 {
            thread::available_parallelism().unwrap().get()
        } else {
            size
        };

        let (task_sender, task_receiver) = mpsc::channel();
        let task_mreceiver = Arc::new(Mutex::new(task_receiver));

        // TODO: figure out how to have multiple senders
        let (result_sender, result_receiver) = mpsc::channel();

        let mut workers = Vec::with_capacity(thread_count);
        for _ in 0..thread_count {
            let result_sender_copy = result_sender.clone();
            workers.push(Worker::new(
                context.clone(),
                task_mreceiver.clone(),
                result_sender_copy,
            ));
        }

        ThreadPool {
            workers,
            tasks: HashSet::new(),
            task_sender: Some(task_sender),
            task_receiver: task_mreceiver.clone(),
            result_receiver,
            has_quit,
        }
    }

    // Send a single run request to the thread pool.
    pub fn run(&mut self, req: engine::RunRequest) -> Result<()> {
        let task_id = Uuid::new_v4();
        self.task_sender
            .as_ref()
            .unwrap()
            .send(Task { id: task_id, req })?;
        self.tasks.insert(task_id);
        Ok(())
    }

    // Wait for all requests to finish running. This function will exit early if
    //
    // This is done by waiting on the result channel for tasks to stream in as they finish. This
    // uses recv_timeout to give the main thread a chance to see if the process has finished.
    pub fn wait(&mut self) -> Result<()> {
        let timeout = time::Duration::from_millis(500);
        while !self.has_quit.load(atomic::Ordering::SeqCst) && !self.tasks.is_empty() {
            match self.result_receiver.recv_timeout(timeout) {
                Ok(task_id) => {
                    self.tasks.remove(&task_id);
                }
                Err(_e) => {
                    continue;
                }
            }
        }
        if self.tasks.is_empty() {
            return Ok(());
        } else {
            return Err(anyhow!("tasks are still remaining"));
        }
    }
}

impl Drop for ThreadPool {
    // Implement graceful shutdown of the workers. This works by closing the task channel, which
    // instructs the workers to stop processing.
    fn drop(&mut self) {
        drop(self.task_sender.take());
        // Additionally drain the channel so the worker doesn't process more tasks
        while let Ok(_) = self.task_receiver.lock().unwrap().try_recv() {}

        for worker in &mut self.workers {
            trace!("[{}] waiting for worker", worker.id);

            if let Some(th) = worker.thread.take() {
                th.join().unwrap();
            }
        }
    }
}

// A single task for the worker.
struct Task {
    id: Uuid,
    req: engine::RunRequest,
}

// A single thread pool worker that accepts files for interpretation and runs them through Deno to
// generate the corresponding IaC.
//
// This worker will spawn a background thread that will pull RunRequests from the given shared
// channel and run it through Deno.
struct Worker {
    id: Uuid,
    thread: Option<thread::JoinHandle<()>>,
}

impl Worker {
    // On construction, spawn the thread for the worker which watches for incoming tasks on the
    // task_receiver channel.
    fn new(
        context: engine::Context,
        task_receiver: Arc<Mutex<mpsc::Receiver<Task>>>,
        result_sender: mpsc::Sender<Uuid>,
    ) -> Worker {
        let id = Uuid::new_v4();
        let thread = thread::spawn(move || {
            let runtime = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .unwrap();

            loop {
                trace!("[{id}] Worker started.");

                let mtask = task_receiver.lock().unwrap().recv();

                match mtask {
                    Ok(task) => {
                        trace!("[{id}] Worker got request to run {}.", task.req);
                        debug!("executing {}", task.req.in_file);

                        if let Err(e) =
                            runtime.block_on(engine::run_js_and_write(&context, &task.req))
                        {
                            error!(
                                "could not execute javascript file `{}`: {e}",
                                task.req.in_file
                            );
                        } else {
                            trace!("[{id}] successfully executed `{}`.", task.req.in_file);
                        }

                        if let Err(e) = result_sender.send(task.id) {
                            error!("could not mark task as done: {e}");
                        }
                    }
                    Err(_) => {
                        trace!("[{id}] Worker disconnected; shutting down.");
                        break;
                    }
                }
            }
        });

        Worker {
            id,
            thread: Some(thread),
        }
    }
}

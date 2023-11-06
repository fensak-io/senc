// Copyright (c) Fensak, LLC.
// SPDX-License-Identifier: MPL-2.0

use std::thread;
use std::sync::{mpsc, Arc, Mutex};

use uuid::Uuid;

use crate::engine;

// A thread pool with a set number of threads to run tasks on.
pub struct ThreadPool {
  workers: Vec<Worker>,
  sender: Option<mpsc::Sender<engine::RunRequest>>,
}

impl ThreadPool {
  // Create a new ThreadPool.
  //
  // The size is the number of threads in the pool. If set to 0, the size will be the number of
  // cores on the machine.
  pub fn new(size: usize) -> ThreadPool {
    let thread_count = if size == 0 {
      thread::available_parallelism().unwrap().get()
    } else {
      size
    };

    let (sender, receiver) = mpsc::channel();
    let mreceiver = Arc::new(Mutex::new(receiver));

    let mut workers = Vec::with_capacity(thread_count);
    for _ in 0..thread_count {
      workers.push(Worker::new(Arc::clone(&mreceiver)));
    }

    ThreadPool { workers, sender: Some(sender) }
  }

  pub fn execute(&self, req: engine::RunRequest) {
    self.sender.as_ref().unwrap().send(req).unwrap();
  }
}

impl Drop for ThreadPool {
  fn drop(&mut self) {
    drop(self.sender.take());

    for worker in &mut self.workers {
      eprintln!("[{}] Shutting down worker", worker.id);

      if let Some(th) = worker.thread.take() {
        th.join().unwrap();
      }
    }
  }
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
  fn new(
    receiver: Arc<Mutex<mpsc::Receiver<engine::RunRequest>>>,
  ) -> Worker {
    let id = Uuid::new_v4();
    let thread = thread::spawn(move || loop {
      eprintln!("[{id}] Worker started.");

      let mreq = receiver.lock().unwrap().recv();

      match mreq {
        Ok(req) => {
          // TODO
          // implement proper project logging
          eprintln!("[{id}] Worker got request to run {req}.");

          let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
          if let Err(e) = runtime.block_on(engine::run_js(&req)) {
            eprintln!("[{id}] could not execute javascript file `{}`: {e}", req.in_file);
          }
        }
        Err(_) => {
          eprintln!("[{id}] Worker disconnected; shutting down.");
          break;
        }
      }
    });

    Worker { id, thread: Some(thread) }
  }
}

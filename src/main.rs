#![feature(array_windows)]

mod directory;
mod dirent;
mod path;
mod syscall;

use directory::opendir;
use dirent::*;
use path::Path;

use crossbeam_channel::TryRecvError;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;
use std::thread;

enum Job {
    GetDirEntries { path: Path },
    Shutdown,
}

struct Shared {
    busy_workers: AtomicU32,
    num_of_workers: AtomicU32,
}

struct Worker {
    shared: Arc<Shared>,
    dirent_buf: Box<[u64]>,
    send: crossbeam_channel::Sender<Job>,
    recv: crossbeam_channel::Receiver<Job>,
    files: Vec<Path>,
    id: u32,
}

impl Worker {
    /// Entry function
    fn work(mut self) -> Vec<Path> {
        eprintln!("Starting worker {}", self.id);
        self.shared.busy_workers.fetch_add(1, Ordering::Relaxed);
        self.shared.num_of_workers.fetch_add(1, Ordering::Relaxed);

        let should_init_shutdown = self.work_loop();

        if should_init_shutdown {
            eprintln!("Shutting down workers");
            // TODO: use some kind of futex logic to wake up all other threads.
            // Wake up all workers, except for ourselves.
            for _ in 1..self.shared.num_of_workers.load(Ordering::Relaxed) {
                self.send.send(Job::Shutdown).unwrap();
            }
        }

        eprintln!("Shutting down worker id={}", self.id);
        self.shared.busy_workers.fetch_sub(1, Ordering::Relaxed);
        return self.files;
    }

    /// After finishing the main loop, returns if the current worker is
    /// responsible for shutting down other workers.
    fn work_loop(&mut self) -> bool {
        loop {
            let job = match self.wait_for_job() {
                Some(j) => j,
                None => return true,
            };

            match job {
                Job::Shutdown => return false,
                Job::GetDirEntries { path } => self.get_dir_entries(path),
            }
        }
    }

    /// Returns None if all jobs are done, Some otherwise.
    fn wait_for_job(&self) -> Option<Job> {
        match self.recv.try_recv() {
            Ok(job) => return Some(job),
            Err(TryRecvError::Disconnected) => panic!("channel disconnected"),
            Err(TryRecvError::Empty) => {} // fallthrough
        }

        // If we were the last busy worker, shutdown all other workers.
        let busy = self.shared.busy_workers.fetch_sub(1, Ordering::SeqCst);
        if busy == 1 && self.recv.is_empty() {
            self.shared.busy_workers.fetch_add(1, Ordering::Relaxed);
            return None;
        }
        let job = self.recv.recv().expect("channel disconnected");
        self.shared.busy_workers.fetch_add(1, Ordering::SeqCst);

        return Some(job);
    }

    fn get_dir_entries(&mut self, path: Path) {
        // We could recycle the allocation, but it doesn't give too much perf.
        let _ = self.walk(path).into_inner();
    }

    fn walk(&mut self, mut path: Path) -> Path {
        let fd = match opendir(&path) {
            Ok(f) => f,
            Err(_) => return path,
        };

        let (dents, _) = getdents64(&fd, &mut self.dirent_buf);
        drop(fd);

        for dent in dents
            .filter(|d| d.filename.to_bytes() != b".")
            .filter(|d| d.filename.to_bytes() != b"..")
        {
            if dent.filename.to_bytes().ends_with(b".png") {
                self.files.push(Path::joined(&path, &dent.filename));
            }

            if dent.typ != DirentType::Directory as u8 {
                continue;
            }

            path.push(&dent.filename);
            self.send
                .send(Job::GetDirEntries { path: path.clone() })
                .unwrap();
            path.truncate(path.to_bytes().len() - dent.filename.to_bytes().len() - 1);
        }

        return path;
    }
}

fn main() {
    const WORKERS: u32 = 4;

    let (s, r) = crossbeam_channel::unbounded();
    s.send(Job::GetDirEntries {
        path: Path::from_str("/"),
    })
    .unwrap();

    let shared = Arc::new(Shared {
        busy_workers: AtomicU32::new(0),
        num_of_workers: AtomicU32::new(0),
    });

    let new_worker = |i: u32| Worker {
        shared: Arc::clone(&shared),
        dirent_buf: vec![0u64; 1024 * 1024].into_boxed_slice(),
        send: s.clone(),
        recv: r.clone(),
        id: i,
        files: Vec::with_capacity(128),
    };

    let workers: Vec<_> = (0..WORKERS)
        .map(new_worker)
        .map(|worker| thread::spawn(move || worker.work()))
        .collect();

    let paths: Vec<Path> = workers
        .into_iter()
        .map(thread::JoinHandle::join)
        .map(Result::unwrap)
        .flatten()
        .collect();

    assert!(r.is_empty());
    assert!(s.is_empty());
    assert_eq!(shared.busy_workers.load(Ordering::Relaxed), 0);

    for path in paths {
        let p = path.as_cstr().to_bytes();
        let p = &p[..p.len() - 1];
        let p = std::str::from_utf8(p).unwrap();
        println!("{}", p);
    }
}

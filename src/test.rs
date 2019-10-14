// Cadence - An extensible Statsd client for Rust!
//
// Copyright 2019 TSH Labs
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! Utilities for testing Cadence itself.
//!
//! Functionality exported to be used by integration tests. This module
//! is NOT part of the Cadence API and is subject to change at any time.
//!
//! IF YOU USE THIS CODE YOUR PROJECT WILL BREAK AND YOU WILL DESERVE IT.

use std::fs;
use std::io::{self, ErrorKind};
use std::os::unix::net::UnixDatagram;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread::JoinHandle;
use std::time::Duration;
use std::{env, thread};

/// Create a temporary directory and construct paths to files within it
///
/// When this object goes out of scope, any files under the temporary directory
/// it is responsible for ($TMP + $PREFIX) will be deleted.
#[derive(Debug)]
pub struct TempDir {
    base: PathBuf,
}

impl TempDir {
    pub fn new<P>(prefix: P) -> io::Result<Self>
    where
        P: AsRef<Path>,
    {
        let base = env::temp_dir().join(prefix);
        fs::create_dir_all(&base)?;
        Ok(TempDir { base })
    }

    pub fn new_path<P>(&self, name: P) -> PathBuf
    where
        P: AsRef<Path>,
    {
        self.base.join(name)
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.base);
    }
}

/// Basic server for listening on a given Unix socket path.
///
/// This server reads messages from a Unix datagram socket in a loop, ensures
/// they are valid UTF-8 strings, and then discards them. Any errors are printed
/// to `stderr`.
///
/// This server is only meant for testing Unix socket related functionality in
/// Cadence itself.
#[derive(Debug)]
pub struct UnixSocketServer {
    ready: AtomicBool,
    shutdown: AtomicBool,
    path: PathBuf,
    interval: Duration,
}

impl UnixSocketServer {
    /// Create a new server that will listen for datagrams on the given path, using
    /// the provided interval on the read timeout as part of its main loop.
    pub fn new<P>(path: P, interval: Duration) -> Self
    where
        P: AsRef<Path>,
    {
        UnixSocketServer {
            ready: AtomicBool::new(false),
            shutdown: AtomicBool::new(false),
            path: path.as_ref().to_path_buf(),
            interval,
        }
    }

    /// Has the server created the socket to listen on?
    pub fn is_ready(&self) -> bool {
        self.ready.load(Ordering::Acquire)
    }

    /// Run until the `.shutdown()` method is called, reading datagrams and discarding them.
    pub fn run(&self) -> io::Result<()> {
        // Make sure to remove any existing socket at the same path before we start
        // listening on it. Ignore any errors since it's entirely possible that the
        // socket file doesn't exist.
        let _ = fs::remove_file(&self.path);
        let socket = UnixDatagram::bind(&self.path)?;
        socket.set_read_timeout(Some(self.interval))?;

        let mut buf = [0u8; 1024];
        self.ready.store(true, Ordering::Release);

        loop {
            match socket.recv(&mut buf) {
                Ok(v) => match std::str::from_utf8(&buf[0..v]) {
                    Ok(_s) => {} // Don't print anything on success because it's too noisy
                    Err(e) => eprintln!("Error: Couldn't decode string to utf-8 {}", e),
                },
                Err(e) => {
                    // WouldBlock means we hit our receive timeout which is expected.
                    // If the "shutdown" flag has been set by the client they've sent
                    // all the metrics they are going to send and we can shutdown the
                    // server. Otherwise, just ignore the WouldBlock error.
                    if e.kind() == ErrorKind::WouldBlock {
                        if self.shutdown.load(Ordering::Acquire) {
                            break;
                        }
                    } else {
                        // Some other kind of error besides hitting our receive timeout
                        eprintln!("Error: {} - {:?}", e, e.kind());
                    }
                }
            }
        }

        Ok(())
    }

    /// Indicate that the server should stop its main run loop.
    pub fn shutdown(&self) {
        self.shutdown.store(true, Ordering::Release);
        eprintln!("UnixSocketServer: shutting down");
    }
}

/// Wrapper around a `UnixSocketServer` to start and stop it in the course
/// of running a single test.
///
/// The server is stopped and the thread it was running in is joined from
/// the destructor of this struct.
pub struct UnixServerHarness {
    base: PathBuf,
    server: Option<Arc<UnixSocketServer>>,
    thread: Option<JoinHandle<()>>,
}

impl UnixServerHarness {
    pub fn new<P>(prefix: P) -> Self
    where
        P: AsRef<Path>,
    {
        UnixServerHarness {
            base: prefix.as_ref().to_path_buf(),
            server: None,
            thread: None,
        }
    }

    /// Run the given closure, providing the path to the Unix socket as an argument.
    pub fn run<F>(mut self, test: F)
    where
        F: FnOnce(&Path) -> (),
    {
        let temp = TempDir::new(&self.base).unwrap();
        let socket = temp.new_path("cadence.sock");

        let server = Arc::new(UnixSocketServer::new(&socket, Duration::from_millis(100)));
        let server_local = Arc::clone(&server);

        let t = thread::spawn(move || {
            server_local.run().unwrap();
        });

        while !server.is_ready() {
            thread::yield_now();
        }

        self.server = Some(server);
        self.thread = Some(t);

        test(&socket);
    }
}

impl Drop for UnixServerHarness {
    fn drop(&mut self) {
        if let Some(s) = self.server.take() {
            s.shutdown();
        }

        if let Some(t) = self.thread.take() {
            let _ = t.join();
        }
    }
}
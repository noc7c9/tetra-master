use owo_colors::OwoColorize;
use std::io::{BufRead, Write};

use crate::{Command, Response};

// Basic Driver that talks to the given Rx, Tx types
struct Driver<Rx, Tx> {
    receiver: Rx,
    transmitter: Tx,
    buffer: String,
    logging: bool,
}

impl<Rx, Tx> Driver<Rx, Tx>
where
    Rx: BufRead,
    Tx: Write,
{
    fn new(receiver: Rx, transmitter: Tx) -> Self {
        Self {
            receiver,
            transmitter,
            buffer: String::new(),
            logging: false,
        }
    }

    fn send(&mut self, cmd: Command) -> anyhow::Result<Response> {
        self.tx(cmd)?;
        self.rx()
    }

    fn tx(&mut self, cmd: Command) -> anyhow::Result<()> {
        self.buffer.clear();
        cmd.serialize(&mut self.buffer)?;

        if self.logging {
            eprint!("{} {}", " TX ".black().on_purple(), self.buffer.purple());
        }

        self.transmitter.write_all(self.buffer.as_bytes())?;
        self.transmitter.flush()?;

        Ok(())
    }

    fn rx(&mut self) -> anyhow::Result<Response> {
        self.buffer.clear();
        self.receiver.read_line(&mut self.buffer)?;

        if self.logging {
            eprint!("{} {}", " RX ".black().on_blue(), self.buffer.blue());
        }

        Response::deserialize(&self.buffer)
    }
}

// Driver for talking to an implementation that's run as an external process
pub struct ImplementationDriver {
    proc: std::process::Child,
    driver: Driver<std::io::BufReader<std::process::ChildStdout>, std::process::ChildStdin>,
    _stderr_thread_handle: std::thread::JoinHandle<()>,
}

impl ImplementationDriver {
    pub fn send(&mut self, cmd: Command) -> anyhow::Result<Response> {
        self.driver.send(cmd)
    }

    pub fn log(mut self) -> Self {
        self.driver.logging = true;
        self
    }
}

impl Drop for ImplementationDriver {
    fn drop(&mut self) {
        // if killing the child fails, just ignore it
        // the OS should clean up after the tester process closes
        let _ = self.proc.kill();
    }
}

impl ImplementationDriver {
    pub fn new(implementation: &str) -> ImplementationDriver {
        use std::process::{Command, Stdio};

        let mut proc = Command::new(implementation)
            .args(["--headless"])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        let stdin = proc.stdin.take().unwrap();

        let stdout = proc.stdout.take().unwrap();
        let stdout = std::io::BufReader::new(stdout);

        // manually handle letting stderr passthrough to ensure output from the driver and the
        // implementation don't get mixed up (at least in the middle of a line)
        let stderr = proc.stderr.take().unwrap();
        let stderr = std::io::BufReader::new(stderr);
        let handle = std::thread::spawn(move || {
            for line in stderr.lines() {
                eprintln!("{}", line.unwrap());
            }
        });

        let driver = Driver::new(stdout, stdin);

        ImplementationDriver {
            proc,
            driver,
            _stderr_thread_handle: handle,
        }
    }
}

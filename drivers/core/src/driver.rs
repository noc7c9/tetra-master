use owo_colors::OwoColorize;
use std::io::{BufRead, Write};

use crate::{
    command::{self, Command},
    response::{self, Response},
    Error, Result,
};

pub trait CommandResponse: Command {
    type Response: Response;
}

impl CommandResponse for command::Setup {
    type Response = response::SetupOk;
}
impl CommandResponse for command::PickHand {
    type Response = response::PickHandOk;
}
impl CommandResponse for command::PlaceCard {
    type Response = response::PlaceCardOk;
}
impl CommandResponse for command::PickBattle {
    type Response = response::PlaceCardOk;
}

// Basic Driver that talks to the given Rx, Tx types
struct BaseDriver<Rx, Tx> {
    receiver: Rx,
    transmitter: Tx,
    buffer: String,
    logging: bool,
}

impl<Rx, Tx> BaseDriver<Rx, Tx>
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

    fn send<C: CommandResponse>(&mut self, cmd: C) -> Result<C::Response> {
        self.tx(cmd)?;
        self.rx()
    }

    fn tx<C: Command>(&mut self, cmd: C) -> Result<()> {
        self.buffer.clear();
        cmd.serialize(&mut self.buffer)?;

        if self.logging {
            eprint!("{} {}", " TX ".black().on_purple(), self.buffer.purple());
        }

        self.transmitter.write_all(self.buffer.as_bytes())?;
        self.transmitter.flush()?;

        Ok(())
    }

    fn rx<R: Response>(&mut self) -> Result<R> {
        self.buffer.clear();
        self.receiver.read_line(&mut self.buffer)?;

        if self.logging {
            eprint!("{} {}", " RX ".black().on_blue(), self.buffer.blue());
        }

        if let Ok(error_response) = response::ErrorResponse::deserialize(&self.buffer) {
            return Err(Error::ErrorResponse(error_response));
        }

        Ok(R::deserialize(&self.buffer)?)
    }
}

// Driver for talking to an implementation that's run as an external process
pub struct Driver {
    proc: std::process::Child,
    base_driver:
        BaseDriver<std::io::BufReader<std::process::ChildStdout>, std::process::ChildStdin>,
    _stderr_thread_handle: std::thread::JoinHandle<()>,
}

impl Driver {
    pub fn send<C: CommandResponse>(&mut self, cmd: C) -> Result<C::Response> {
        self.base_driver.send(cmd)
    }

    pub fn log(mut self) -> Self {
        self.base_driver.logging = true;
        self
    }
}

impl Drop for Driver {
    fn drop(&mut self) {
        // if killing the child fails, just ignore it
        // the OS should clean up after the tester process closes
        let _ = self.proc.kill();
    }
}

impl Driver {
    pub fn new(implementation: &str) -> Driver {
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

        let base_driver = BaseDriver::new(stdout, stdin);

        Driver {
            proc,
            base_driver,
            _stderr_thread_handle: handle,
        }
    }
}

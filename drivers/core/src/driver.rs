use crate::{
    command::Command,
    ref_impl::{ReferenceImplementation, Step},
    response::{ErrorResponse, Response},
    CommandResponse, Error, Result,
};

pub enum Driver {
    External(ExternalDriver),
    Reference(ReferenceDriver),
}

impl Driver {
    pub fn external(implementation: &str) -> Self {
        Self::External(ExternalDriver::new(implementation))
    }

    pub fn reference() -> Self {
        Self::Reference(ReferenceDriver::new())
    }

    pub fn send<C>(&mut self, cmd: C) -> Result<C::Response>
    where
        C: CommandResponse + Step + std::fmt::Debug,
        C::Response: std::fmt::Debug,
    {
        match self {
            Self::External(d) => d.send(cmd),
            Self::Reference(d) => d.send(cmd),
        }
    }

    pub fn log(mut self) -> Self {
        match &mut self {
            Self::External(d) => d.log(),
            Self::Reference(d) => d.log(),
        }
        self
    }
}

// Talks to the embedded reference implementation
pub struct ReferenceDriver {
    ref_impl: ReferenceImplementation,
    logging: bool,
}

impl ReferenceDriver {
    fn new() -> Self {
        Self {
            ref_impl: ReferenceImplementation::new(),
            logging: false,
        }
    }

    fn log(&mut self) {
        self.logging = true;
    }

    fn send<C>(&mut self, cmd: C) -> Result<C::Response>
    where
        C: CommandResponse + Step + std::fmt::Debug,
        C::Response: std::fmt::Debug,
    {
        use owo_colors::OwoColorize;

        if self.logging {
            eprintln!("{} {cmd:?}", " TX ".black().on_purple());
        }

        let res = self.ref_impl.step(cmd);

        if self.logging {
            eprintln!("{} {res:?}", " RX ".black().on_blue());
        }

        res
    }
}

// Talks to an implementation that's run as an external process
pub struct ExternalDriver {
    proc: std::process::Child,

    receiver: std::io::BufReader<std::process::ChildStdout>,
    transmitter: std::process::ChildStdin,

    buffer: String,
    logging: bool,

    _stderr_thread_handle: std::thread::JoinHandle<()>,
}

impl ExternalDriver {
    fn new(implementation: &str) -> Self {
        use std::process::{Command, Stdio};

        let mut proc = Command::new(implementation)
            .args(["--headless"])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();

        let proc_stdin = proc.stdin.take().unwrap();

        let proc_stdout = proc.stdout.take().unwrap();
        let proc_stdout = std::io::BufReader::new(proc_stdout);

        // manually handle letting stderr passthrough to ensure output from the driver and the
        // implementation don't get mixed up (at least in the middle of a line)
        let proc_stderr = proc.stderr.take().unwrap();
        let proc_stderr = std::io::BufReader::new(proc_stderr);
        let thread_handle = std::thread::spawn(move || {
            use std::io::BufRead;
            for line in proc_stderr.lines() {
                eprintln!("{}", line.unwrap());
            }
        });

        Self {
            proc,

            receiver: proc_stdout,
            transmitter: proc_stdin,

            buffer: String::new(),
            logging: false,

            _stderr_thread_handle: thread_handle,
        }
    }

    fn log(&mut self) {
        self.logging = true;
    }

    fn send<C: CommandResponse>(&mut self, cmd: C) -> Result<C::Response> {
        self.tx(cmd)?;
        self.rx()
    }

    fn tx<C: Command>(&mut self, cmd: C) -> Result<()> {
        use owo_colors::OwoColorize;
        use std::io::Write;

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
        use owo_colors::OwoColorize;
        use std::io::BufRead;

        self.buffer.clear();
        self.receiver.read_line(&mut self.buffer)?;

        if self.logging {
            eprint!("{} {}", " RX ".black().on_blue(), self.buffer.blue());
        }

        if let Ok(error_response) = ErrorResponse::deserialize(&self.buffer) {
            return Err(Error::ErrorResponse(error_response));
        }

        Ok(R::deserialize(&self.buffer)?)
    }
}

impl Drop for ExternalDriver {
    fn drop(&mut self) {
        // if killing the child fails, just ignore it
        // the OS should clean up after the tester process closes
        let _ = self.proc.kill();
    }
}

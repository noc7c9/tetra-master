use crate::{
    command::{self, Command},
    random_setup,
    ref_impl::{ReferenceImplementation, Step},
    response::{self, ErrorResponse, Response},
    BattleSystem, CommandResponse, Error, Result, Rng, Seed,
};

pub struct Driver {
    inner: Inner,
    log: bool,
    pub rng: Rng,
}

enum Inner {
    External(ExternalDriver),
    Reference(ReferenceDriver),
}

impl Driver {
    pub fn external(implementation: &str) -> DriverBuilder {
        let inner = Inner::External(ExternalDriver::new(implementation));
        DriverBuilder::new(inner)
    }

    pub fn reference() -> DriverBuilder {
        let inner = Inner::Reference(ReferenceDriver::new());
        DriverBuilder::new(inner)
    }

    pub fn get_rng(&mut self) -> &mut Rng {
        &mut self.rng
    }

    pub fn random_setup(&mut self, battle_system: BattleSystem) -> command::Setup {
        let blocked_cells = random_setup::random_blocked_cells(&mut self.rng);
        let [hand_blue, hand_red] = random_setup::random_hands(&mut self.rng);
        let starting_player = random_setup::random_starting_player(&mut self.rng);
        command::Setup {
            battle_system,
            blocked_cells,
            hand_blue,
            hand_red,
            starting_player,
        }
    }

    pub fn send_random_setup(&mut self, battle_system: BattleSystem) -> Result<response::SetupOk> {
        let cmd = self.random_setup(battle_system);
        self.send(cmd)
    }

    pub fn resolve_battle(&mut self, requests: response::ResolveBattle) -> command::ResolveBattle {
        fn fulfill(rng: &mut Rng, request: response::RandomNumberRequest) -> Vec<u8> {
            let mut nums = Vec::with_capacity(request.numbers as usize);
            for _ in 0..request.numbers {
                let (min, max) = request.range;
                nums.push(rng.gen_range(min..=max));
            }
            nums
        }
        command::ResolveBattle {
            attack_roll: fulfill(&mut self.rng, requests.attack_roll),
            defend_roll: fulfill(&mut self.rng, requests.defend_roll),
        }
    }

    pub fn send_resolve_battle(
        &mut self,
        requests: response::ResolveBattle,
    ) -> Result<response::PlayOk> {
        let cmd = self.resolve_battle(requests);
        self.send(cmd)
    }

    pub fn send<C>(&mut self, cmd: C) -> Result<C::Response>
    where
        C: CommandResponse + Step + std::fmt::Display,
        C::Response: std::fmt::Display,
    {
        use owo_colors::OwoColorize;

        if self.log {
            let prefix = " TX ".black().on_bright_purple();
            log::info!("{} {}", prefix, cmd.bright_purple());
        }

        let res = match &mut self.inner {
            Inner::External(d) => d.send(cmd),
            Inner::Reference(d) => d.send(cmd),
        };

        if self.log {
            let prefix = " RX ".black().on_bright_blue();
            match &res {
                Ok(ok) => log::info!("{} {}", prefix, ok.bright_blue()),
                Err(err) => log::info!("{} Err {}", prefix, err.bright_blue()),
            }
        }

        res
    }
}

pub struct DriverBuilder {
    inner: Inner,
    log: bool,
    seed: Option<Seed>,
}

impl DriverBuilder {
    fn new(inner: Inner) -> Self {
        Self {
            inner,
            log: true,
            seed: None,
        }
    }

    pub fn no_log(mut self) -> Self {
        self.log = false;
        self
    }

    pub fn seed(mut self, seed: impl Into<Option<Seed>>) -> Self {
        self.seed = seed.into();
        self
    }

    pub fn build(mut self) -> Driver {
        let rng = self.seed.map_or_else(Rng::new, Rng::from_seed);

        if self.log {
            if log::log_enabled!(log::Level::Info) {
                use owo_colors::OwoColorize;
                use std::fmt::Write;

                let mut s = String::new();
                let _ = write!(s, "{} Initializing ", "INIT".black().on_green());
                let _ = match &self.inner {
                    Inner::External(inner) => {
                        let implementation = inner.implementation.green();
                        write!(s, "{} ({implementation})", "External Driver".green(),)
                    }
                    Inner::Reference(_) => write!(s, "{}", "Reference Driver".green()),
                };
                let _ = write!(s, " | Seed: {}", rng.initial_seed.green());
                log::info!("{s}");
            }

            // turn on logging for the internal logger,
            // this logs the raw protocol
            if let Inner::External(inner) = &mut self.inner {
                inner.log = true;
            }
        }

        Driver {
            inner: self.inner,
            rng,
            log: self.log,
        }
    }
}

// Talks to the embedded reference implementation
struct ReferenceDriver {
    ref_impl: ReferenceImplementation,
}

impl ReferenceDriver {
    fn new() -> Self {
        Self {
            ref_impl: ReferenceImplementation::new(),
        }
    }

    fn send<C>(&mut self, cmd: C) -> Result<C::Response>
    where
        C: CommandResponse + Step + std::fmt::Display,
        C::Response: std::fmt::Display,
    {
        self.ref_impl.step(cmd)
    }
}

// Talks to an implementation that's run as an external process
struct ExternalDriver {
    implementation: String,
    proc: std::process::Child,

    receiver: std::io::BufReader<std::process::ChildStdout>,
    transmitter: std::process::ChildStdin,
    buffer: String,

    log: bool,

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
            implementation: implementation.to_string(),
            proc,

            receiver: proc_stdout,
            transmitter: proc_stdin,

            buffer: String::new(),
            log: false,

            _stderr_thread_handle: thread_handle,
        }
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

        if self.log {
            log::info!("{} {}", " TX ".black().on_purple(), self.buffer.purple());
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

        if self.log {
            log::info!("{} {}", " RX ".black().on_blue(), self.buffer.blue());
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

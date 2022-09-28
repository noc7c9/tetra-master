use crate::{
    command::{self, Command},
    random_setup,
    ref_impl::{ReferenceImplementation, Step},
    response,
    response::{ErrorResponse, Response},
    BattleSystem, CommandResponse, Error, Result,
};
use rand::{thread_rng, Rng as _, SeedableRng as _};

pub type Seed = u64;
pub type Rng = rand_pcg::Pcg32;

const MIN_RNG_NUMBERS: usize = 256;

pub struct Driver {
    inner: Inner,
    pub initial_seed: Seed,
    rng: Rng,
    auto_feed_rng: bool,
    log: bool,
    prev_rng_numbers_left: usize,
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

    pub fn send_random_setup(&mut self, battle_system: BattleSystem) -> Result<response::SetupOk> {
        let blocked_cells = random_setup::random_blocked_cells(&mut self.rng);
        let hand_candidates = random_setup::random_hand_candidates(&mut self.rng);
        self.send(command::Setup {
            battle_system,
            blocked_cells,
            hand_candidates,
        })
    }

    pub fn send<C>(&mut self, cmd: C) -> Result<C::Response>
    where
        C: CommandResponse + Step + std::fmt::Display,
        C::Response: std::fmt::Display,
    {
        let res = self.send_actual(cmd);
        self.auto_feed_rng()?;
        res
    }

    fn auto_feed_rng(&mut self) -> Result<()> {
        if !self.auto_feed_rng {
            return Ok(());
        };

        let count = MIN_RNG_NUMBERS.saturating_sub(self.prev_rng_numbers_left);

        let mut numbers = Vec::with_capacity(count);
        for _ in 0..count {
            numbers.push(self.rng.gen());
        }

        let ok = self.send_actual(command::PushRngNumbers { numbers })?;
        self.prev_rng_numbers_left = ok.numbers_left;

        Ok(())
    }

    fn send_actual<C>(&mut self, cmd: C) -> Result<C::Response>
    where
        C: CommandResponse + Step + std::fmt::Display,
        C::Response: std::fmt::Display,
    {
        use owo_colors::OwoColorize;

        if self.log {
            let prefix = " TX ".black().on_bright_purple();
            eprintln!("{} {}", prefix, cmd.bright_purple());
        }

        let res = match &mut self.inner {
            Inner::External(d) => d.send(cmd),
            Inner::Reference(d) => d.send(cmd),
        };

        if self.log {
            let prefix = " RX ".black().on_bright_blue();
            match &res {
                Ok(ok) => eprintln!("{} {}", prefix, ok.bright_blue()),
                Err(err) => eprintln!("{} Err {}", prefix, err.bright_blue()),
            }
        }

        res
    }
}

pub struct DriverBuilder {
    inner: Inner,
    auto_feed_rng: bool,
    log: bool,
    seed: Option<Seed>,
}

impl DriverBuilder {
    fn new(inner: Inner) -> Self {
        Self {
            inner,
            auto_feed_rng: true,
            log: false,
            seed: None,
        }
    }

    pub fn auto_feed_rng(mut self) -> Self {
        self.auto_feed_rng = true;
        self
    }

    pub fn no_auto_feed_rng(mut self) -> Self {
        self.auto_feed_rng = false;
        self
    }

    pub fn log(mut self) -> Self {
        self.log = true;
        self
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
        let initial_seed = self.seed.unwrap_or_else(|| thread_rng().gen());

        if self.log {
            use owo_colors::OwoColorize;

            eprint!("{} Initializing ", "INIT".black().on_green());
            match &self.inner {
                Inner::External(inner) => {
                    eprint!(
                        "{} ({})",
                        "External Driver".green(),
                        inner.implementation.green()
                    )
                }
                Inner::Reference(_) => eprint!("{}", "Reference Driver".green()),
            }
            eprint!(" | Seed: {}", initial_seed.green());
            if self.auto_feed_rng {
                eprint!(" | Auto Feed Rng: {}", "On".green())
            }
            eprintln!();

            // turn on logging for the internal logger,
            // this logs the raw protocol
            if let Inner::External(inner) = &mut self.inner {
                inner.log = true;
            }
        }

        let rng = Rng::seed_from_u64(initial_seed);
        Driver {
            inner: self.inner,
            initial_seed,
            rng,
            auto_feed_rng: self.auto_feed_rng,
            log: self.log,
            prev_rng_numbers_left: 0,
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

        if self.log {
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

#[cfg(test)]
mod tests {
    use super::*;

    fn get_rng_numbers_len(driver: &Driver) -> usize {
        if let Inner::Reference(inner) = &driver.inner {
            return match &inner.ref_impl {
                ReferenceImplementation::PreSetup(inner) => {
                    inner.rng.as_ref().unwrap().numbers.len()
                }
                ReferenceImplementation::PickingHands(inner) => inner.rng.numbers.len(),
                ReferenceImplementation::InGame(inner) => inner.rng.numbers.len(),
            };
        }
        unreachable!()
    }

    #[test]
    fn should_push_more_random_numbers_after_running_each_command() -> Result<()> {
        let mut driver = Driver::reference().seed(0).log().build();

        // immediately after initialization it should be empty
        assert_eq!(get_rng_numbers_len(&driver), 0);

        driver.send_random_setup(BattleSystem::Original)?;

        // after setup command, rng should be auto fed
        assert_eq!(get_rng_numbers_len(&driver), MIN_RNG_NUMBERS);

        // doesn't use any numbers
        driver.send(command::PickHand { hand: 0 })?;
        driver.send(command::PickHand { hand: 1 })?;
        driver.send(command::PlaceCard { card: 0, cell: 10 })?;

        assert_eq!(get_rng_numbers_len(&driver), MIN_RNG_NUMBERS);

        // triggers a battle and uses 4 numbers
        driver.send(command::PlaceCard { card: 3, cell: 5 })?;
        assert_eq!(get_rng_numbers_len(&driver), MIN_RNG_NUMBERS - 4);

        // doesn't use any numbers, but numbers should be refilled
        driver.send(command::PlaceCard { card: 1, cell: 0 })?;
        assert_eq!(get_rng_numbers_len(&driver), MIN_RNG_NUMBERS);

        Ok(())
    }

    #[test]
    fn should_not_push_more_random_numbers_if_auto_feed_rng_is_off() -> Result<()> {
        let mut driver = Driver::reference().seed(0).log().no_auto_feed_rng().build();

        // immediately after initialization it should be empty
        assert_eq!(get_rng_numbers_len(&driver), 0);

        driver.send_random_setup(BattleSystem::Original)?;

        // after setup command, it should still be empty
        assert_eq!(get_rng_numbers_len(&driver), 0);

        Ok(())
    }
}

use owo_colors::OwoColorize;
use std::io::{BufRead, Write};

mod command;
mod response;

pub(crate) use command::Command;
pub(crate) use response::Response;

pub(crate) struct Driver<Rx, Tx> {
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
    pub(crate) fn new(receiver: Rx, transmitter: Tx) -> Self {
        Self {
            receiver,
            transmitter,
            buffer: String::new(),
            logging: false,
        }
    }

    pub(crate) fn send(&mut self, cmd: Command) -> anyhow::Result<Response> {
        self.tx(cmd)?;
        self.rx()
    }

    #[allow(dead_code)]
    pub(crate) fn toggle_logging(&mut self) {
        self.logging = !self.logging;
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

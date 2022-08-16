use std::io::{BufRead, Write};

mod command;
mod response;

pub(crate) use command::Command;
pub(crate) use response::Response;

pub(crate) struct Driver<Rx, Tx> {
    receiver: Rx,
    transmitter: Tx,
    buffer: String,
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
        }
    }

    pub(crate) fn transmit(&mut self, cmd: Command) -> anyhow::Result<()> {
        self.buffer.clear();
        cmd.serialize(&mut self.buffer)?;

        self.transmitter.write_all(self.buffer.as_bytes())?;
        self.transmitter.flush()?;

        Ok(())
    }

    pub(crate) fn receive(&mut self) -> anyhow::Result<Response> {
        self.buffer.clear();
        self.receiver.read_line(&mut self.buffer)?;

        Response::deserialize(&self.buffer)
    }
}

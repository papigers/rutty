use std::{pin::Pin, sync::Arc};

use async_stream::stream;
use bytes::Bytes;
use futures_util::Stream;
use pty_process::{OwnedWritePty, Pty, Size};
use thiserror::Error;
use tokio::{process::Child, sync::Notify};
use tokio_stream::StreamExt;
use tokio_util::io::ReaderStream;
use tracing::{debug, error};

pub enum CommandStreamItem {
    Output(Bytes),
    Error(String),
    Exit(String),
}

#[derive(Debug, Error)]
pub enum CommandError {
    #[error("Failed to start command: {0}")]
    Error(String),
}

pub type CommandStream = Pin<Box<dyn Stream<Item = CommandStreamItem> + Send>>;
pub type CommandWriter = OwnedWritePty;

pub struct Command {
    inner: pty_process::Command,
    pty: Option<Pty>,
    child: Option<Child>,
}

#[derive(Debug, Clone, Copy)]
pub struct TerminalSize(pub u16, pub u16);

impl TerminalSize {
    pub fn rows(&self) -> u16 {
        self.0
    }
    pub fn cols(&self) -> u16 {
        self.1
    }
}

impl Into<Size> for TerminalSize {
    fn into(self) -> Size {
        Size::new(self.rows(), self.cols())
    }
}

impl Command {
    pub fn new(program: &str) -> Command {
        Command {
            inner: pty_process::Command::new(program),
            pty: None,
            child: None,
        }
    }

    pub fn args<I, S>(&mut self, args: I) -> &mut Self
    where
        I: IntoIterator<Item = S>,
        S: AsRef<std::ffi::OsStr>,
    {
        self.inner.args(args);
        self
    }

    pub fn spawn(&mut self, size: Option<TerminalSize>) -> Result<(), CommandError> {
        let pty = match Pty::new() {
            Ok(it) => it,
            Err(e) => return Result::Err(CommandError::Error(e.to_string())),
        };

        if let Some(size) = size {
            pty.resize(size.into()).ok();
        }

        let pts = &pty.pts();
        let pts = match pts {
            Ok(it) => it,
            Err(e) => return Result::Err(CommandError::Error(e.to_string())),
        };

        let child = match self.inner.spawn(&pts) {
            Ok(it) => it,
            Err(e) => return Result::Err(CommandError::Error(e.to_string())), // return;
        };

        self.pty = Some(pty);
        self.child = Some(child);

        Ok(())
    }

    pub fn read_and_control(self, aborter: Arc<Notify>) -> (CommandWriter, CommandStream) {
        let (pty_out, pty_in) = self.pty.unwrap().into_split();
        let mut child = self.child.unwrap();
        let mut out_stream = ReaderStream::new(pty_out);

        let stream = futures_util::StreamExt::boxed(stream! {
            loop {
                tokio::select! {
                    Some(output) = out_stream.next() =>
                        match output {
                            Ok(b) => yield CommandStreamItem::Output(b.into()),
                            // workaround against PTY closing incorrect error handling
                            // see: https://stackoverflow.com/questions/72150987/why-does-reading-from-an-exited-pty-process-return-input-output-error-in-rust
                            Err(err) if err.to_string() == "Input/output error (os error 5)" => continue,
                            Err(err) => yield CommandStreamItem::Error(err.to_string()),
                        },
                    status = child.wait() => {
                        match status {
                            Err(err) => yield CommandStreamItem::Error(err.to_string()),
                            Ok(status) => {
                                let code = status.code().unwrap_or(0);
                                yield CommandStreamItem::Exit(format!("Command exited with status code: {code}"));
                                break;
                            }
                        }
                    },
                    _ = aborter.notified() => {
                        match child.start_kill() {
                            Ok(()) => debug!("Command aborted"),
                            Err(err) => error!("Failed to abort command: {err}"),
                        };
                        yield CommandStreamItem::Exit("Aborted".to_string());
                        break;
                    }
                }
            }
        });

        (pty_in, stream)
    }

    pub fn pid(&self) -> Option<u32> {
        self.child.as_ref().unwrap().id()
    }
}

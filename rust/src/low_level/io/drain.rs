use crate::low_level::io::buffer::BufferState;
use crate::low_level::io::writer::WriterState;
use crate::low_level::reactor::{Fd, Token};
use crate::low_level::spawn::SysError;

pub struct FdSlot {
    pub token: Option<Token>,
    pub fd: Fd,
}

#[repr(align(64))]
pub struct DrainState<F>
where
    F: FnMut(&[u8]) -> bool,
{
    pub stdout_slot: Option<FdSlot>,
    pub stderr_slot: Option<FdSlot>,
    pub stdin_slot: Option<FdSlot>,

    pub buffer: BufferState,
    pub writer: WriterState,

    pub early_exit: Option<F>,
}

impl<F> DrainState<F>
where
    F: FnMut(&[u8]) -> bool,
{
    pub fn new(
        _job_id: u64,
        stdin_fd: Option<Fd>,
        stdin_buf: Option<Box<[u8]>>,
        stdout_fd: Option<Fd>,
        stderr_fd: Option<Fd>,
        limit: usize,
        early_exit: Option<F>,
    ) -> Result<Self, SysError> {
        let mut stdin_slot = None;
        let mut stdout_slot = None;
        let mut stderr_slot = None;

        // Tokens remain purely unassigned until explicitly mapped by a Reactor
        if let (Some(fd), Some(_)) = (&stdin_fd, &stdin_buf) {
            fd.set_nonblock()?;
            stdin_slot = Some(FdSlot {
                token: None,
                fd: stdin_fd.unwrap(),
            });
        }

        if let Some(fd) = &stdout_fd {
            fd.set_nonblock()?;
            stdout_slot = Some(FdSlot {
                token: None,
                fd: stdout_fd.unwrap(),
            });
        }

        if let Some(fd) = &stderr_fd {
            fd.set_nonblock()?;
            stderr_slot = Some(FdSlot {
                token: None,
                fd: stderr_fd.unwrap(),
            });
        }

        Ok(Self {
            stdin_slot,
            stdout_slot,
            stderr_slot,
            buffer: BufferState::new(limit),
            writer: WriterState::new(stdin_buf),
            early_exit,
        })
    }

    #[inline(always)]
    pub fn is_done(&self) -> bool {
        self.stdin_slot.is_none() && self.stdout_slot.is_none() && self.stderr_slot.is_none()
    }

    #[inline(always)]
    pub fn write_stdin(&mut self) -> Result<Option<FdSlot>, SysError> {
        let fd = if let Some(s) = &self.stdin_slot {
            &s.fd
        } else {
            return Ok(None);
        };

        let done = self.writer.write_to_fd(fd)?;
        if done {
            let slot = self.stdin_slot.take();
            return Ok(slot);
        }
        Ok(None)
    }

    #[inline(always)]
    pub fn read_fd(&mut self, is_stdout: bool) -> Result<Option<FdSlot>, SysError> {
        let eof = {
            let slot = if is_stdout {
                &self.stdout_slot
            } else {
                &self.stderr_slot
            };
            let fd = if let Some(s) = slot {
                &s.fd
            } else {
                return Ok(None);
            };
            self.buffer
                .read_from_fd(fd, is_stdout, &mut self.early_exit)?
        };

        if eof {
            if is_stdout {
                let slot = self.stdout_slot.take();
                return Ok(slot);
            } else {
                let slot = self.stderr_slot.take();
                return Ok(slot);
            }
        }

        Ok(None)
    }

    pub fn take_all_slots(&mut self) -> Vec<FdSlot> {
        let mut slots = Vec::new();
        if let Some(slot) = self.stdin_slot.take() {
            slots.push(slot);
        }
        if let Some(slot) = self.stdout_slot.take() {
            slots.push(slot);
        }
        if let Some(slot) = self.stderr_slot.take() {
            slots.push(slot);
        }
        slots
    }

    pub fn into_parts(mut self) -> (Vec<u8>, Vec<u8>) {
        std::mem::take(&mut self.buffer).into_parts()
    }
}

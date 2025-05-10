use std::error::Error;
use std::fmt;
use std::pin::Pin;
use std::task::{Context, Poll, Waker};

#[macro_export]
macro_rules! task {
    ($task:expr) => {
        Box::pin($task)
    };
}

pub type Task<Return> = Pin<Box<dyn Future<Output = Return>>>;

#[derive(Debug)]
pub enum SansIoError {
    InvalidPollPending,
}

impl fmt::Display for SansIoError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidPollPending => {
                write!(f, "Task encountered a Poll::Pending response.")
            }
        }
    }
}

impl Error for SansIoError {}

/// Attempt to run a synchronous task
///
/// # Errors
///
/// It is an error if this task returns `Poll::Pending`.
pub fn try_run<Return>(mut task: Task<Return>) -> Result<Return, SansIoError> {
    let waker = Waker::noop();
    let mut ctx = Context::from_waker(waker);

    match task.as_mut().as_mut().poll(&mut ctx) {
        Poll::Ready(ret) => Ok(ret),
        Poll::Pending => Err(SansIoError::InvalidPollPending),
    }
}

/// Run a task, panicing on `SansIoErrors`
///
/// # Panics
///
/// Any task that returns `Poll::Pending` will cause this function to panic.
#[must_use]
pub fn run<Return>(task: Task<Return>) -> Return {
    match try_run(task) {
        Ok(ret) => ret,
        Err(err) => panic!("{}", err),
    }
}

use std::cell::RefCell;
use std::fmt;
use std::pin::Pin;
use std::task::{Context, Poll};

#[macro_export]
macro_rules! task {
    ($expr:expr) => {
        Box::pin($expr)
    };
}

pub type Task<'task, Return> = Pin<Box<dyn Future<Output = Return> + 'task>>;

#[derive(Debug)]
pub enum Error {
    InvalidInvocation,
    InvalidResponse,
    InvalidStepAttempt,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::InvalidInvocation => {
                write!(f, "Attempted to invoke a call from an invalid state.")
            }
            Error::InvalidResponse => {
                write!(
                    f,
                    "Attempted to respond to a call from an invalid state."
                )
            }
            Error::InvalidStepAttempt => {
                write!(f, "Attempted to advance a step from an invalid state.")
            }
        }
    }
}

impl std::error::Error for Error {}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum State {
    Ready,
    Calling,
    Responding,
    Responded,
}

pub struct SansIo<Api> {
    op: RefCell<Option<Api>>,
    st: RefCell<State>,
}

impl<Api> SansIo<Api> {
    pub fn new() -> Self {
        Self {
            op: Default::default(),
            st: RefCell::new(State::Ready),
        }
    }

    pub fn invoke(
        &self,
        api: Api,
    ) -> Result<impl Future<Output = Api> + use<'_, Api>, Error> {
        if !self.in_state(State::Ready) {
            return Err(Error::InvalidInvocation);
        }

        assert!(self.op.borrow().is_none());

        self.op.borrow_mut().replace(api);
        self.transition(State::Ready, State::Calling);
        Ok(SansIoFuture::new(self))
    }

    pub fn respond(&self, api: Api) -> Result<(), Error> {
        if !self.in_state(State::Responding) {
            return Err(Error::InvalidResponse);
        }

        assert!(self.op.borrow().is_none());
        self.op.borrow_mut().replace(api);

        self.transition(State::Responding, State::Responded);

        Ok(())
    }

    fn transition(&self, from: State, to: State) {
        assert_eq!(*self.st.borrow(), from);
        *self.st.borrow_mut() = to;
    }

    fn in_state(&self, st: State) -> bool {
        return st == *self.st.borrow();
    }

    fn take_request(&self) -> Api {
        assert!(self.in_state(State::Calling));
        self.transition(State::Calling, State::Responding);
        // The unwrap is our assertion that we have an operation to
        // deliver after the task yielded.
        self.op.borrow_mut().take().unwrap()
    }
}

impl<Api> Default for SansIo<Api> {
    fn default() -> Self {
        Self::new()
    }
}

struct SansIoFuture<'sio, Api> {
    sio: &'sio SansIo<Api>,
}

impl<'sio, Api> SansIoFuture<'sio, Api> {
    fn new(sio: &'sio SansIo<Api>) -> Self {
        assert!(sio.in_state(State::Calling));
        Self { sio }
    }
}

impl<Api> Future for SansIoFuture<'_, Api> {
    type Output = Api;

    fn poll(self: Pin<&mut Self>, context: &mut Context) -> Poll<Self::Output> {
        if self.sio.in_state(State::Calling) {
            // All tasks should signal the waker before pending. Even though
            // we're using the no-op waker, its still good practice.
            context.waker().wake_by_ref();
            Poll::Pending
        } else if self.sio.in_state(State::Responded) {
            self.sio.transition(State::Responded, State::Ready);
            Poll::Ready(self.sio.op.borrow_mut().take().unwrap())
        } else {
            panic!("Invalid sans-io state.");
        }
    }
}

pub enum Step<Api, Return> {
    Call(Api),
    Return(Return),
}

pub struct Driver<'task, Return> {
    ctx: Context<'static>,
    task: Task<'task, Return>,
}

impl<'task, Return> Driver<'task, Return> {
    pub fn new(task: Task<'task, Return>) -> Self {
        let waker = std::task::Waker::noop();
        Self {
            ctx: Context::from_waker(waker),
            task,
        }
    }

    pub fn step<Api>(
        &mut self,
        sio: &SansIo<Api>,
    ) -> Result<Step<Api, Return>, Error> {
        if !(sio.in_state(State::Ready) || sio.in_state(State::Responded)) {
            return Err(Error::InvalidStepAttempt);
        }

        match self.task.as_mut().as_mut().poll(&mut self.ctx) {
            Poll::Ready(ret) => {
                assert!(sio.in_state(State::Ready));
                Ok(Step::Return(ret))
            }
            Poll::Pending => Ok(Step::Call(sio.take_request())),
        }
    }
}

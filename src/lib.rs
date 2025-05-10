use std::cell::RefCell;
use std::fmt;
use std::pin::Pin;
use std::rc::Rc;
use std::task::{Context, Poll};

#[macro_export]
macro_rules! task {
    ($task:expr, $sm:expr) => {
        Box::pin($task($sm.dupe()))
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

pub struct SansIo<Args, Returns = Args> {
    args: Rc<RefCell<Option<Args>>>,
    ret: Rc<RefCell<Option<Returns>>>,
    st: Rc<RefCell<State>>,
}

impl<Args, Returns> SansIo<Args, Returns> {
    pub fn new() -> Self {
        Self {
            args: Default::default(),
            ret: Default::default(),
            st: Rc::new(RefCell::new(State::Ready)),
        }
    }

    pub fn dupe(&self) -> Self {
        Self {
            args: Rc::clone(&self.args),
            ret: Rc::clone(&self.ret),
            st: Rc::clone(&self.st),
        }
    }

    pub fn invoke(
        &self,
        args: Args,
    ) -> Result<impl Future<Output = Returns> + use<'_, Args, Returns>, Error>
    {
        if !self.in_state(State::Ready) {
            return Err(Error::InvalidInvocation);
        }

        assert!(self.args.borrow().is_none());
        assert!(self.ret.borrow().is_none());

        self.args.borrow_mut().replace(args);
        self.transition(State::Ready, State::Calling);
        Ok(SansIoFuture::new(self))
    }

    pub fn respond(&self, ret: Returns) -> Result<(), Error> {
        if !self.in_state(State::Responding) {
            return Err(Error::InvalidResponse);
        }

        assert!(self.args.borrow().is_none());
        assert!(self.ret.borrow().is_none());
        self.ret.borrow_mut().replace(ret);

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

    fn take_request(&self) -> Args {
        assert!(self.in_state(State::Calling));
        self.transition(State::Calling, State::Responding);
        // The unwrap is our assertion that we have an operation to
        // deliver after the task yielded.
        self.args.borrow_mut().take().unwrap()
    }
}

impl<Args, Returns> Default for SansIo<Args, Returns> {
    fn default() -> Self {
        Self::new()
    }
}

struct SansIoFuture<'sio, Args, Returns> {
    sio: &'sio SansIo<Args, Returns>,
}

impl<'sio, Args, Returns> SansIoFuture<'sio, Args, Returns> {
    fn new(sio: &'sio SansIo<Args, Returns>) -> Self {
        assert!(sio.in_state(State::Calling));
        Self { sio }
    }
}

impl<Args, Returns> Future for SansIoFuture<'_, Args, Returns> {
    type Output = Returns;

    fn poll(self: Pin<&mut Self>, context: &mut Context) -> Poll<Self::Output> {
        if self.sio.in_state(State::Calling) {
            // All tasks should signal the waker before pending. Even though
            // we're using the no-op waker, its still good practice.
            context.waker().wake_by_ref();
            Poll::Pending
        } else if self.sio.in_state(State::Responded) {
            self.sio.transition(State::Responded, State::Ready);
            Poll::Ready(self.sio.ret.borrow_mut().take().unwrap())
        } else {
            panic!("Invalid sans-io state.");
        }
    }
}

pub enum Step<Api, Return> {
    Next(Api),
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

    pub fn step<Args, Returns>(
        &mut self,
        sio: &SansIo<Args, Returns>,
    ) -> Result<Step<Args, Return>, Error> {
        if !(sio.in_state(State::Ready) || sio.in_state(State::Responded)) {
            return Err(Error::InvalidStepAttempt);
        }

        match self.task.as_mut().as_mut().poll(&mut self.ctx) {
            Poll::Ready(ret) => {
                assert!(sio.in_state(State::Ready));
                Ok(Step::Return(ret))
            }
            Poll::Pending => Ok(Step::Next(sio.take_request())),
        }
    }
}

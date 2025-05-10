use sans_io as sio;

#[derive(Debug, thiserror::Error)]
enum ApiError {
    #[error("Error interacting with sans-io: {0}")]
    Sio(#[from] sio::Error),

    #[error(transparent)]
    Io(#[from] std::io::Error),
}

#[derive(Debug)]
struct SomeBigStruct {
    val: usize,
}

impl SomeBigStruct {
    fn get_val(&self) -> usize {
        self.val
    }
}

pub struct Args {}
pub struct Returns(usize);

fn main() -> Result<(), ApiError> {
    let sbs = SomeBigStruct { val: 42 };

    let async_closure =
        async |sm: sio::SansIo<Args, Returns>| -> Result<usize, ApiError> {
            // Call sbs directly
            assert_eq!(sbs.get_val(), 42);

            // Defer a call if it's slow/blocking
            let Returns(val) = sm.invoke(Args {})?.await;
            assert_eq!(val, 42);
            Ok(42)
        };

    let sm: sio::SansIo<Args, Returns> = sio::SansIo::new();
    let mut driver = sio::Driver::new(sio::task!(async_closure, sm));
    let ret = loop {
        match driver.step(&sm)? {
            sio::Step::Next(Args {}) => {
                sm.respond(Returns(sbs.get_val()))?;
                continue;
            }
            sio::Step::Return(ret) => break ret?,
        }
    };

    println!("Yay, we got {ret} out!");

    Ok(())
}

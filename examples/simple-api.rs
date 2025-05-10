use sans_io as sio;

#[derive(Debug)]
enum Api {
    Args(usize, String),
    Return(bool),
}

impl Api {
    async fn do_thing(
        sm: &sio::SansIo<Api>,
        u: usize,
        s: String,
    ) -> Result<bool, sio::Error> {
        match sm.invoke(Api::Args(u, s))?.await {
            Api::Return(ret) => Ok(ret),
            _ => panic!("Invalid return value."),
        }
    }

    fn handle(&self) -> Self {
        if let Self::Args(u, s) = self {
            Self::Return(format!("{u}") == *s)
        } else {
            panic!("Invalid handle call.")
        }
    }
}

fn main() -> Result<(), sio::Error> {
    let sm: sio::SansIo<Api> = sio::SansIo::new();
    let mut driver = sio::Driver::new(sio::task!(make_call, sm));
    let ret = loop {
        match driver.step(&sm)? {
            sio::Step::Next(call) => {
                sm.respond(call.handle())?;
                continue;
            }
            sio::Step::Return(ret) => break ret?,
        }
    };

    println!("Yay, we got {ret} out!");

    Ok(())
}

async fn make_call(sm: sio::SansIo<Api>) -> Result<usize, sio::Error> {
    assert!(Api::do_thing(&sm, 42, "42".into()).await?);
    assert!(!Api::do_thing(&sm, 84, "42".into()).await?);

    Ok(42)
}

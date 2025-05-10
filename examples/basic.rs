use sans_io as sio;

#[derive(Debug)]
enum Api {
    Request,
    Response,
}

fn main() -> Result<(), sio::Error> {
    let sm: sio::SansIo<Api> = sio::SansIo::new();
    let mut driver = sio::Driver::new(sio::task!(make_call, sm));
    let ret = loop {
        match driver.step(&sm)? {
            sio::Step::Next(call) => {
                println!("Our task made a call! {call:?}");
                sm.respond(Api::Response)?;
                continue;
            }
            sio::Step::Return(ret) => break ret?,
        }
    };

    println!("Yay, we got {ret} out!");

    Ok(())
}

async fn make_call(sm: sio::SansIo<Api>) -> Result<usize, sio::Error> {
    println!("Lets make a call!");

    let val = sm.invoke(Api::Request)?.await;

    println!("Our call returned: {val:?}");

    Ok(42)
}

fn main() {
    println!("Still trying to figure out the lifetimes");
}

// use sans_io as sio;

// #[derive(Debug)]
// pub enum Args<'s> {
//     DoThing(usize, &'s str),
//     DoOtherThing(&'s str),
// }

// #[derive(Debug)]
// enum Returns {
//     DoThing(bool),
//     DoOtherThing(String),
// }

// impl<'s> Args<'s> {
//     async fn do_thing(
//         sm: &sio::SansIo<Args<'s>, Returns>,
//         u: usize,
//         s: &'s str,
//     ) -> Result<bool, sio::Error> {
//     }

//     async fn do_other_thing(
//         sm: &sio::SansIo<Args<'s>, Returns>,
//         s: &'s str,
//     ) -> Result<String, sio::Error> {
//         match sm.invoke(Args::DoOtherThing(s))?.await {
//             Returns::DoOtherThing(ret) => Ok(ret),
//             _ => panic!("Invalid return value."),
//         }
//     }

//     fn handle(&self) -> Returns {
//         match self {
//             Args::DoThing(_, _) => self.handle_do_thing(),
//             Args::DoOtherThing(_) => self.handle_do_other_thing(),
//         }
//     }

//     fn handle_do_thing(&self) -> Returns {
//         if let Self::DoThing(u, s) = self {
//             Returns::DoThing(format!("{u}") == *s)
//         } else {
//             panic!("Invalid handle call.")
//         }
//     }

//     fn handle_do_other_thing(&self) -> Returns {
//         if let Self::DoOtherThing(s) = self {
//             Returns::DoOtherThing(s.to_string())
//         } else {
//             panic!("Invalid handle call.");
//         }
//     }
// }

// fn main() -> Result<(), sio::Error> {
//     let sm: sio::SansIo<Args, Returns> = sio::SansIo::new();
//     let mut driver = sio::Driver::new(sio::task!(make_call, sm));
//     let ret = loop {
//         match driver.step(&sm)? {
//             sio::Step::Next(call) => {
//                 sm.respond(call.handle())?;
//                 continue;
//             }
//             sio::Step::Return(ret) => break ret?,
//         }
//     };

//     println!("Yay, we got {ret} out!");

//     Ok(())
// }

// async fn make_call(
//     sm: sio::SansIo<Args<'_>, Returns>,
// ) -> Result<usize, sio::Error> {
//     assert!(Args::do_thing(&sm, 42, "42").await?);

//     let value = "42".to_string();
//     let ret = match sm.invoke(Args::DoThing(42, &value))?.await {
//         Returns::DoThing(ret) => ret,
//         _ => panic!("Invalid return value."),
//     };

//     assert!(ret);

//     Ok(42)
// }

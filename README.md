# Sans-Io

This crate contains a synchronous executor for `Future`'s. Its a bit complicated
under the hood, but the basic idea is that there are two functions that accept
a box'ed `Future` and either returns an `Error` or `panic!`'s. Code written
with the `async` keyword can then use traits to allow for the same code to run
in either a synchronous or asynchronous context.

And when I say its complicated, believe me. It's almost as bad as the manually
derived Error trait to avoid an uncessary dependency on thiserror.

## What is sans-io?

Its event driven programming. It's older than I am. And I'm getting up there.
I'm sure someone somewhere knows the history better than me. If someone opens
an issue with book recomendations on the history of event driven programming I'd
be more than happy to drop a link or reference here.

But the basic point is, we've spent years and years trying to hide the fact that
pretty much all of computering and programmering is one massive attempt to hide
the fact that _all_ software is event driven. That's just how the hardware
works.

If you're getting the feeling that I'm not terribly fond of event driven
programming, you'd be absolutely correct. Its a terrible approach to software
engineering due to how it forces programmers to chop up their logic into
discrete chunks which removes linearity. I consider advocating for event driven
programming (sorry, "sans io") similar to folks that would advocate that we
should all go back to writing assembly by hand. Obviously, no one serious
thinks we should be writing everything in assembly, that's what compilers are
for after all.

Which is exactly my point! Compilers can take our lovely linear logic and apply
the necessary transformations that make them fit into an event driven framework.
Similar to assembly, I'm sure there are places where dropping down to event
driven programming might possibly eke out some measurable performance benefit.
But just like assembly, it should be done after the first implementation
so that there's at least a baseline to measure against.

## Function Colors and Rust

For anyone thinking about colored functions at this point, I urge you to go
re-read the [blog post][colors] and consider two things. First, the author
specifically points out that `async`/`await` were a good enough compromise that
they were adding them to Dart. The second thing to consider is that most folks
worry about how they have to get an async context down to where they need to
make an `async` call.

That part is easy. You just add an `async` to the function that needs to
`await`. Then just follow the call stack up until you get to main and you're
done. For anyone about to say "But that's the entire point! I have to add
async and await everywhere!" I will direct your attention to the other end of
the function declaration where it probably says `Result` or `Option`. We're all
propogating our errors up the call stack, what's the big deal about piping an
async context down the stack with a couple keywords?

## Is this crate a joke?

Mostly. Check out the implementation, its not actually complicated at all.

## So why did you create it?

Because [this blog post][firezone] from the folks at Firezone has annoyed me
for long enough that I got the urge to write something about it. And that's
absolutely not a ding at the Firezone folks. Its a very well written article
and demonstrates the concepts quite nicely. The part that I found madening was
switching back to assembly when we have the tools to avoid this completely just
sitting there with the `async` keyword.

In hindsight, I really wish the language designers had picked a different
keyword for triggering the compiler transformation. I'm not sure what could
have been used, but its just unfortunate that folks see `async` and then
immediately think of Tokio and runtimes and way way more than the keyword
actually implies.

So I wrote up this crate and an example of what it could look like if we just
used the async keyword with a custom executor instead.

## So what does that look like?

The other reason I picked the Firezone folks' blog post is that their's was
the only one that had enough code that I could easily implement the whole
thing based on what was written. Lots of other folks just described the
approach without a concrete implementations. So, hats off to the Firezone folks'
for that.

So, assuming my reading comprehension is up to the task, this is the final
logic that we get from the post. Send a packet, wait 5s, if no packet, send
another. If we receive a packet, parse it and return the IP. Seems easy enough.

```rs
async fn get_public_ip(sock: impl Socket) -> Result<String> {
    sock.connect("stun.cloudflare.com:3478").await?;

    let req = utils::make_binding_request()??
    let mut resp = vec![0u8; 1024];
    loop {
        sock.send(&req).await?;

        match sock.recv(&mut resp, 5000).await {
            Ok(num_read) => {
                resp.resize(num_read, 0);
                return utils::parse_binding_response(&resp);
            }
            Err(err) if err.kind() == ErrorKind::TimedOut => {
                continue;
            }
            err => err?,
        };
    }
}
```

You'll notice that I have a `Socket` trait there to abstract out the calls that
might need to be async. I'll note that using an `async` function doesn't
absolutely require an interface like this as long as its not calling something
that's _actually_ async. Connecting, reading, and writing sockets are absolutely
operations that need to be async-able.

Anywho, here's the `Socket` definition.

```rs
trait Socket {
    async fn connect(&self, addr: &str) -> IoResult<()>;
    async fn send(&self, buf: &[u8]) -> IoResult<usize>;
    async fn recv(&self, buf: &mut [u8], timeout: u64) -> IoResult<usize>;
}
```

And now for the fun part. I'll start with the tokio implementation as I'm
sure most folks will understand that we're basically just proxying through
other than the timeout being odd.

```rs
impl Socket for UdpSocket {
    async fn connect(&self, addr: &str) -> IoResult<()> {
        self.connect(addr).await
    }

    async fn send(&self, buf: &[u8]) -> IoResult<usize> {
        self.send(buf).await
    }

    async fn recv(&self, buf: &mut [u8], timeout: u64) -> IoResult<usize> {
        let mut timer = Box::pin(sleep(Duration::from_millis(timeout)));
        tokio::select! {
            () = &mut timer => {
                Err(Error::new(ErrorKind::TimedOut, "recv timed out"))
            },
            res = self.recv(buf) => res,
        }
    }
}

#[tokio::main]
pub async fn get_public_ip() -> Result<()> {
    let sock = UdpSocket::bind("0.0.0.0:0").await?;

    let ip = super::get_public_ip(sock).await?;
    println!("Async IP: {ip}");

    Ok(())
}
```

That's hopefully pretty straight forward. Just proxy through to the tokio
types. Nothing fancy going on there.

The synchronous version is similarly simple. Even more so with the timeout
code.

```rs
impl Socket for UdpSocket {
    async fn connect(&self, addr: &str) -> IoResult<()> {
        self.connect(addr)
    }

    async fn send(&self, buf: &[u8]) -> IoResult<usize> {
        self.send(buf)
    }

    async fn recv(&self, buf: &mut [u8], timeout: u64) -> IoResult<usize> {
        self.set_read_timeout(Some(Duration::from_millis(timeout)))?;
        self.recv(buf)
    }
}

pub fn get_public_ip() -> Result<()> {
    let sock = UdpSocket::bind("0.0.0.0:0")?;

    let task = sans_io::task!(super::get_public_ip(sock));
    let ip = sans_io::run(task)?;

    println!("Sync IP: {ip}");

    Ok(())
}
```

[colors]: https://journal.stuffwithstuff.com/2015/02/01/what-color-is-your-function/
[firezone]: https://www.firezone.dev/blog/sans-io

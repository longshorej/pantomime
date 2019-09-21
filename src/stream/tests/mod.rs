use crate::actor::*;
use crate::stream::*;
use std::time::Duration;

#[test]
fn test() {
    use crate::actor::*;
    use crate::stream::flow::Delay;
    use std::task::Poll;

    fn double_slowly(n: u64) -> u64 {
        let start = std::time::Instant::now();

        while start.elapsed().as_millis() < 250 {}

        n * 2
    }

    struct TestReaper {
        n: usize,
    }

    impl TestReaper {
        fn new() -> Self {
            Self { n: 0 }
        }
    }

    impl Actor<()> for TestReaper {
        fn receive(&mut self, _: (), ctx: &mut ActorContext<()>) {
            self.n += 1;

            println!("n={}", self.n);

            if self.n == 2 {
                ctx.stop();
            }
        }

        fn receive_signal(&mut self, signal: Signal, ctx: &mut ActorContext<()>) {
            match signal {
                Signal::Started => {
                    let (stream_ref, result) = ctx.spawn(
                        Source::iterator(1..=20)
                            .via(Flow::new(Delay::new(Duration::from_millis(50))))
                            .to(Sink::for_each(|n| println!("got {}", n))),
                    );

                    ctx.watch(stream_ref, |_: StopReason| ());
                    ctx.watch(result, |value| value);
                }

                _ => {}
            }
        }
    }

    assert!(ActorSystem::new().spawn(TestReaper::new()).is_ok());
}

#[test]
fn test2() {
    use crate::actor::*;
    use crate::stream::flow::Delay;
    use std::io::{Error, ErrorKind};

    struct TestReaper {
        n: usize,
    }

    impl TestReaper {
        fn new() -> Self {
            Self { n: 0 }
        }
    }

    impl Actor<usize> for TestReaper {
        fn receive(&mut self, value: usize, ctx: &mut ActorContext<usize>) {
            self.n += value;

            if self.n == 101 {
                ctx.stop();
            }
        }

        fn receive_signal(&mut self, signal: Signal, ctx: &mut ActorContext<usize>) {
            match signal {
                Signal::Started => {
                    {
                        let actor_ref = ctx.actor_ref().clone();

                        ctx.schedule_thunk(Duration::from_secs(10), move || {
                            actor_ref
                                .fail(FailureError::new(Error::new(ErrorKind::Other, "failed")))
                        });
                    }

                    let (stream_ref, result) = ctx.spawn(
                        Source::iterator(1..=100_000_000)
                            .via(Flow::new(Delay::new(Duration::from_millis(50))))
                            .to(Sink::first()),
                    );

                    ctx.watch(stream_ref, |_: StopReason| 100);
                    ctx.watch(result, |value: Option<usize>| value.unwrap_or_default());
                }

                _ => {}
            }
        }
    }

    assert!(ActorSystem::new().spawn(TestReaper::new()).is_ok());
}
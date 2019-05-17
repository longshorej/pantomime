/// WIP
/// WIP
/// WIP
/// WIP
/// WIP
use crate::actor::ActorRef;
use crate::stream::detached::*;
use crate::stream::sink::tell::{TellEvent, TellHandle};
use crate::stream::*;
use std::marker::PhantomData;

pub struct Merge<A, Peer: Stage<A>>
where
    A: 'static + Send,
{
    source: Option<Peer>,
    pulled: bool,
    pulled_one: bool,
    pulled_two: bool,
    handle: Option<TellHandle<A>>,
    buffer: [Option<A>; 2],
    phantom: PhantomData<A>,
}

impl<A, Peer: Stage<A>> Merge<A, Peer>
where
    A: 'static + Send,
{
    pub fn new(peer: Peer) -> Self {
        Self {
            source: Some(peer),
            pulled: false,
            pulled_one: false,
            pulled_two: false,
            handle: None,
            buffer: [None, None],
            phantom: PhantomData,
        }
    }

    fn store(&mut self, elem: A) {
        if self.buffer[0].is_none() {
            self.buffer[0] = Some(elem);
        } else if self.buffer[1].is_none() {
            self.buffer[1] = Some(elem);
        } else {
            panic!("TODO")
        }
    }

    fn try_push(&mut self) -> Option<AsyncAction<A, TellEvent<A>>> {
        if self.pulled && self.buffer[0].is_some() {
            println!("psuH!");
            let elem = self.buffer[0]
                .take()
                .expect("pantomime bug: Merge#try_push, Option#is_some lied");

            self.buffer[0] = self.buffer[1].take();

            Some(AsyncAction::Push(elem))
        } else if self.pulled && self.buffer[1].is_some() {
            panic!("pantomime bug: Merge#try_push buffer[1] was not empty while buffer[0] was");
        } else {
            None
        }
    }
}

impl<A, Peer: Stage<A>> DetachedLogic<A, A, TellEvent<A>> for Merge<A, Peer>
where
    A: 'static + Send,
{
    fn attach(
        &mut self,
        context: &StreamContext,
        actor_ref: &ActorRef<AsyncAction<A, TellEvent<A>>>,
    ) -> Option<AsyncAction<A, TellEvent<A>>> {
        let stream = self
            .source
            .take()
            .expect("cannot call Merge#attach twice")
            .to(Sinks::tell(actor_ref.convert(|m| AsyncAction::Forward(m))));

        context.spawn_stream(stream);

        None
    }

    fn forwarded(&mut self, msg: TellEvent<A>) -> Option<AsyncAction<A, TellEvent<A>>> {
        match msg {
            TellEvent::Started(handle) => {
                println!("started!");
                handle.pull();

                self.try_push()
            }

            TellEvent::Produced(elem, handle) => {
                println!("produced");
                self.store(elem);

                self.handle = Some(handle);

                self.try_push()
            }

            TellEvent::Completed => panic!("TODO"),

            TellEvent::Failed(error) => panic!("TODO"),
        }
    }

    fn produced(&mut self, elem: A) -> Option<AsyncAction<A, TellEvent<A>>> {
        self.store(elem);

        self.try_push()
    }

    fn pulled(&mut self) -> Option<AsyncAction<A, TellEvent<A>>> {
        self.pulled = true;

        println!("pulled!");

        self.try_push()
    }

    fn completed(&mut self) -> Option<AsyncAction<A, TellEvent<A>>> {
        Some(AsyncAction::Complete)
    }

    fn failed(&mut self, error: Error) -> Option<AsyncAction<A, TellEvent<A>>> {
        Some(AsyncAction::Fail(error))
    }
}
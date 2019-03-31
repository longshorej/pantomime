/*use crate::dispatcher::Dispatcher;
use crate::stream::*;
use std::marker::PhantomData;

pub struct Identity<A, Up: Producer<A>, Down: Consumer<A>>
where
    A: 'static + Send,
{
    upstream: Up,
    downstream: Down,
    runtime: ProducerRuntime,
    phantom: PhantomData<A>,
}

impl<A, Up> Identity<A, Up, Disconnected>
where
    A: 'static + Send,
    Up: Producer<A>,
{
    pub fn new() -> impl FnOnce(Up) -> Self {
        move |upstream| Self {
            upstream: upstream,
            downstream: Disconnected,
            phantom: PhantomData,
            runtime: ProducerRuntime::new(),
        }
    }
}

impl<A, U, D> Producer<A> for Identity<A, U, D>
where
    A: 'static + Send,
    U: Producer<A>,
    D: Consumer<A>,
{
    fn attach<Consume: Consumer<A>>(self, consumer: Consume, system: ActorSystemContext) -> Bounce<Completed> {
        self.runtime.setup(context.dispatcher.safe_clone());

        self.upstream.tell(ProducerCommand::Attach(
            Identity {
                upstream: Disconnected,
                downstream: consumer,
                runtime: self.runtime,
                phantom: PhantomData,
            },
            context.clone(),
        ));
    }

    fn request<Consume: Consumer<A>>(self, consumer: Consume, demand: usize) -> Bounce<Completed> {

    }

    fn cancel<Consume: Consumer<A>>(self, consumer: Consume) -> Bounce<Completed> {

    }

    fn receive<Consume: Consumer<A>>(mut self, command: ProducerCommand<A, Consume>) -> Bounce<Completed> {
        match command {
            ProducerCommand::Attach(consumer, context) => {

            }

            ProducerCommand::Cancel(consumer, _) => {
                self.upstream.tell(ProducerCommand::Cancel(
                    Identity {
                        upstream: Disconnected,
                        downstream: consumer,
                        runtime: self.runtime,
                        phantom: PhantomData,
                    },
                    None,
                ));
            }

            ProducerCommand::Request(consumer, demand) => {
                self.upstream.tell(ProducerCommand::Request(
                    Identity {
                        upstream: Disconnected,
                        downstream: consumer,
                        runtime: self.runtime,
                        phantom: PhantomData,
                    },
                    demand,
                ));
            }
        }

        Bounce::Done(Completed)
    }

    fn runtime(&mut self) -> Option<&mut ProducerRuntime> {
        Some(&mut self.runtime)
    }
}

impl<A, U, D> Consumer<A> for Identity<A, U, D>
where
    A: 'static + Send,
    U: Producer<A>,
    D: Consumer<A>,
{
    fn receive<Produce: Producer<A>>(self, event: ProducerEvent<A, Produce>) -> Bounce<Completed> {
        match event {
            ProducerEvent::Produced(producer, element) => {
                self.downstream.tell(ProducerEvent::Produced(
                    Identity {
                        upstream: producer,
                        downstream: Disconnected,
                        runtime: self.runtime,
                        phantom: PhantomData,
                    },
                    element,
                ));
            }

            ProducerEvent::Started(producer) => {
                self.downstream.tell(ProducerEvent::Started(Identity {
                    upstream: producer,
                    downstream: Disconnected,
                    runtime: self.runtime,
                    phantom: PhantomData,
                }));
            }

            ProducerEvent::Completed => {
                self.downstream.tell::<Self>(ProducerEvent::Completed);
            }

            ProducerEvent::Failed(e) => {
                self.downstream.tell::<Self>(ProducerEvent::Failed(e));
            }
        }

        Bounce::Done(Completed)
    }
}
*/
use crate::stream::{Action, Logic, LogicEvent, StreamContext};

pub struct Filter<F> {
    filter: F,
}

impl<F> Filter<F> {
    pub fn new<A>(filter: F) -> Self
    where
        F: FnMut(&A) -> bool,
    {
        Self { filter }
    }
}

impl<A: Send, F: FnMut(&A) -> bool + Send> Logic<A, A> for Filter<F> {
    type Ctl = ();

    fn name(&self) -> &'static str {
        "Filter"
    }

    fn receive(
        &mut self,
        msg: LogicEvent<A, Self::Ctl>,
        _: &mut StreamContext<A, A, Self::Ctl>,
    ) -> Action<A, Self::Ctl> {
        match msg {
            LogicEvent::Pushed(element) => {
                if (self.filter)(&element) {
                    Action::Push(element)
                } else {
                    Action::Pull
                }
            }

            LogicEvent::Pulled => Action::Pull,
            LogicEvent::Cancelled => Action::Cancel,
            LogicEvent::Stopped => Action::Stop(None),
            LogicEvent::Started => Action::None,
            LogicEvent::Forwarded(()) => Action::None,
        }
    }
}

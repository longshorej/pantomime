use self::actor_watcher::{ActorWatcher, ActorWatcherMessage};
use super::*;
use crate::dispatcher::WorkStealingDispatcher;
use crossbeam::channel;
use fern::colors::{Color, ColoredLevelConfig};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Once};
use std::{time, usize};

static INITIALIZE_ONCE: Once = Once::new();

/// The top level type that contains references for running actors.
///
/// Among other things, each `ActorSystem` is comprised of a number of
/// shards that actors are assigned to, as well as a global dispatcher
/// for scheduling and executing actors.
///
/// Effectively, an `ActorSystem` is a collection of references to the
/// various datastructures that support the execution of its actors.
pub struct ActorSystem {}

/// Holds references to the system's configuration, global dispatcher,
/// and various internal data structures.
pub struct ActorSystemContext {
    pub config: ActorSystemConfig,
    pub dispatcher: WorkStealingDispatcher,
    next_id: AtomicUsize,
    timer_ref: Option<ActorRef<TimerMsg>>,
    pub(in crate::actor) watcher_ref: Option<ActorRef<ActorWatcherMessage>>,
    system_shard: Arc<ActorShard>,
    shards: Vec<Arc<ActorShard>>,
    sender: channel::Sender<ActorSystemMsg>,
}

impl ActorSystemContext {
    fn new(
        config: ActorSystemConfig,
        dispatcher: WorkStealingDispatcher,
        initial_id: usize,
        timer_ref: Option<ActorRef<TimerMsg>>,
        watcher_ref: Option<ActorRef<ActorWatcherMessage>>,
        sender: channel::Sender<ActorSystemMsg>,
        shards: Option<usize>,
    ) -> Self {
        let shards = {
            let mut shards = Vec::with_capacity(shards.unwrap_or_else(|| config.shards()));

            for _ in 0..shards.capacity() {
                shards.push(Arc::new(ActorShard::new()));
            }

            shards
        };

        let system_shard = Arc::new(ActorShard::new());

        Self {
            config,
            dispatcher,
            next_id: AtomicUsize::new(initial_id),
            timer_ref,
            watcher_ref,
            system_shard,
            shards,
            sender,
        }
    }

    pub fn drain(&self) {
        let _ = self.sender.send(ActorSystemMsg::Drain);
    }

    pub fn stop(&self) {
        let _ = self.sender.send(ActorSystemMsg::Stop);
    }

    /// Schedule a function to be invoked after the timeout has elapsed.
    ///
    /// The supplied function will be executed on the system dispatcher.
    ///
    /// Internally, this uses a wheel-based timer that by default can schedule tasks upto a
    /// granularity of 10 milliseconds.
    pub fn schedule_thunk<F: FnOnce()>(&self, timeout: time::Duration, f: F)
    where
        F: 'static + Send + Sync,
    {
        if let Some(ref timer_ref) = self.timer_ref {
            timer_ref.tell(TimerMsg::Schedule {
                after: timeout,
                thunk: TimerThunk::new(Box::new(move || f())),
            });
        } else {
            panic!("pantomime bug: schedule_thunk called on internal context");
        }
    }

    #[allow(dead_code)]
    pub(crate) fn spawn_anonymous_actor<M: 'static + Send, A: 'static + Send>(
        context: &Arc<Self>,
        actor: A,
    ) -> ActorRef<M>
    where
        A: Actor<M>,
    {
        // @TODO doesnt seem right
        let parent_ref = SystemActorRef {
            id: 0,
            scheduler: Box::new(NoopActorRefScheduler),
        };

        Self::spawn_actor(context, ActorType::Root, actor, parent_ref)
    }

    pub(in crate::actor) fn spawn_actor<M: 'static + Send, A: 'static + Send>(
        context: &Arc<Self>,
        actor_type: ActorType,
        actor: A,
        parent_ref: SystemActorRef,
    ) -> ActorRef<M>
    where
        A: Actor<M>,
    {
        let actor_id = context.next_id.fetch_add(1, Ordering::SeqCst);

        let custom_dispatcher = actor.config_dispatcher(context);

        let mut custom_mailbox = actor.config_mailbox(context);

        let custom_mailbox_appender = match custom_mailbox {
            None => None,
            Some(ref mut m) => Some(m.appender()),
        };

        let shard = match custom_dispatcher {
            None => {
                let s = context.shards.len();

                if s == 0 {
                    context.system_shard.clone()
                } else {
                    context.shards[actor_id % s].clone()
                }
            }

            Some(d) => Arc::new(ActorShard::new().with_dispatcher(d)),
        };

        let actor_context = ActorContext::new();

        let cell = Arc::new(ActorCell::new(
            actor,
            parent_ref,
            actor_context,
            custom_mailbox,
        ));

        let actor_ref = ActorRef::new(Arc::new(ActorRefInner {
            actor_type,
            id: actor_id,
            new_cell: Arc::downgrade(&cell),
            shard,
            system_context: context.clone(),
            custom_mailbox_appender,
        }));

        let mut contents = cell.contents.lock();

        contents.store(cell.clone());

        contents.initialize(actor_ref.clone());

        actor_ref
    }
}

// @TODO signaled
enum ActorSystemMsg {
    Drain,
    Stop,
    Done,
}

pub struct ActiveActorSystem {
    pub context: Arc<ActorSystemContext>,
    receiver: channel::Receiver<ActorSystemMsg>,
    sender: channel::Sender<ActorSystemMsg>,
}

impl ActiveActorSystem {
    fn new(
        context: Arc<ActorSystemContext>,
        receiver: channel::Receiver<ActorSystemMsg>,
        sender: channel::Sender<ActorSystemMsg>,
    ) -> Self {
        Self {
            context,
            receiver,
            sender,
        }
    }

    /// Process system messages, taking over the thread in the process.
    ///
    /// For most applications, this will be the final call in main.
    ///
    /// This will return once the actor system has stopped, which happens
    /// after a request to stop/drain has occurred, and all root actors
    /// have terminated.
    ///
    /// If your application requires some special processing on main thread
    /// (e.g. some graphics libraries), this should be called in its own
    /// thread instead.
    pub fn join(self) {
        #[allow(unused_mut)]
        let mut exit_code = 0;

        #[cfg(feature = "posix-signals-support")]
        use signal_hook::iterator::Signals;

        #[cfg(feature = "posix-signals-support")]
        let signals = Signals::new(&self.context.config.posix_signals)
            .expect("pantomime bug: cannot setup POSIX signal handling");

        // this provides a mechanism to occasionally check for signals that
        // have arrived (if posix signal support is enabled). alternatively,
        // an extra thread could be spawned, but that doesn't seem worth it
        // at the moment
        //
        // it's not currently configurable as there shouldn't really be
        // a desire to do so. if lower resolution latency is required,
        // please open an issue explaining the use-case

        let time_duration = time::Duration::from_millis(100);

        loop {
            #[cfg(feature = "posix-signals-support")]
            for signal in signals.pending() {
                if let Some(ref watcher_ref) = self.context.watcher_ref {
                    watcher_ref.tell(ActorWatcherMessage::ReceivedPosixSignal(signal));
                }

                if self.context.config.posix_shutdown_signals.contains(&signal) {
                    let _ = self.sender.send(ActorSystemMsg::Drain);
                    exit_code = 128 + signal;
                }
            }

            match self.receiver.recv_timeout(time_duration) {
                Err(channel::RecvTimeoutError::Timeout) => {
                    // nothing to do
                }

                Err(channel::RecvTimeoutError::Disconnected) => {
                    break;
                }

                Ok(ActorSystemMsg::Drain) => {
                    if let Some(ref watcher_ref) = self.context.watcher_ref {
                        let sender = self.sender.clone();

                        watcher_ref.tell(ActorWatcherMessage::DrainSystem(Box::new(move || {
                            if let Err(_e) = sender.send(ActorSystemMsg::Done) {
                                // @TODO handle _e
                            }
                        })));
                    } else {
                        panic!(
                            "pantomime bug: received drain request for actor without watcher_ref"
                        );
                    }
                }

                Ok(ActorSystemMsg::Stop) => {
                    if let Some(ref watcher_ref) = self.context.watcher_ref {
                        let sender = self.sender.clone();

                        watcher_ref.tell(ActorWatcherMessage::StopSystem(Box::new(move || {
                            if let Err(e) = sender.send(ActorSystemMsg::Done) {
                                // there's nothing to do here -- the receiver has been dropped
                                // since the message was sent to the watcher

                                drop(e);
                            }
                        })));
                    } else {
                        panic!(
                            "pantomime bug: received stop request for actor without watcher_ref"
                        );
                    }
                }

                Ok(ActorSystemMsg::Done) => {
                    // our watcher has indicated that all non-system actors have stopped,
                    // so we're done. we have to stop the timer as it's a special case and
                    // has its own thread that it needs to stop. in general, if other
                    // "special" actors are added in the future, they should be stopped here
                    // too. stopping is on a best-effort basis but should usually succeed

                    if let Some(ref timer_ref) = self.context.timer_ref {
                        timer_ref.tell(TimerMsg::Stop);
                    }

                    break;
                }
            }
        }

        #[cfg(test)]
        let _ = exit_code;

        #[cfg(not(test))]
        let _ = {
            if self.context.config.process_exit {
                ::std::process::exit(exit_code);
            }
        };
    }

    pub fn spawn<M: 'static + Send, A: 'static + Send>(&mut self, actor: A) -> ActorRef<M>
    where
        A: Actor<M>,
    {
        // @TODO use a real parent

        let parent_ref = SystemActorRef {
            id: 0,
            scheduler: Box::new(NoopActorRefScheduler),
        };

        ActorSystemContext::spawn_actor(&self.context, ActorType::Root, actor, parent_ref)
    }
}

impl ActorSystem {
    pub fn new() -> Self {
        Self {}
    }

    pub fn start(self) -> ActiveActorSystem {
        INITIALIZE_ONCE.call_once(|| {
            if let Some(log_err) = Self::setup_logger().err() {
                panic!("pantomime bug: cannot initialize logger; {}", log_err);
            }
        });

        let config = ActorSystemConfig::parse();

        let dispatcher = WorkStealingDispatcher::new(
            config.default_dispatcher_parallelism(),
            config.default_dispatcher_task_queue_fifo,
        );

        let (sender, receiver) = channel::unbounded();

        let system_context = Arc::new(ActorSystemContext::new(
            config.clone(),
            dispatcher.clone(),
            usize::MIN + 100, // we reserve < 100 as an internal id, i.e. special. in practice, we currently only need 2
            None,
            None,
            sender.clone(),
            Some(0),
        ));

        let parent_ref = SystemActorRef {
            id: 0,
            scheduler: Box::new(NoopActorRefScheduler),
        };

        let ticker_interval = time::Duration::from_millis(system_context.config.ticker_interval_ms);

        let timer_ref = ActorSystemContext::spawn_actor(
            &system_context,
            ActorType::System,
            Timer::new(ticker_interval),
            parent_ref.clone(),
        );

        let actor_watcher_ref = ActorSystemContext::spawn_actor(
            &system_context,
            ActorType::System,
            ActorWatcher::new(),
            parent_ref,
        );

        let context = Arc::new(ActorSystemContext::new(
            config.clone(),
            dispatcher.clone(),
            1,
            Some(timer_ref.clone()),
            Some(actor_watcher_ref),
            sender.clone(),
            None,
        ));

        if context.config.log_config_on_start {
            info!("configuration: {:?}", context.config);
        }

        // @TODO use a real parent

        ActiveActorSystem::new(context, receiver, sender)
    }

    fn setup_logger() -> Result<(), fern::InitError> {
        let mut colors = ColoredLevelConfig::new();
        colors.info = Color::Blue;
        let tty = atty::is(atty::Stream::Stderr);

        fern::Dispatch::new()
            .format(move |out, message, record| {
                if tty {
                    out.finish(format_args!(
                        "{} {} [{}] {}",
                        chrono::Local::now().to_rfc3339(),
                        colors.color(record.level()),
                        record.target(),
                        message
                    ))
                } else {
                    out.finish(format_args!(
                        "{} {} [{}] {}",
                        chrono::Local::now().to_rfc3339(),
                        record.level(),
                        record.target(),
                        message
                    ))
                }
            })
            .level(log::LevelFilter::Info)
            .chain(std::io::stderr())
            .apply()?;
        Ok(())
    }
}

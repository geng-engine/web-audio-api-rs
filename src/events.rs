use crate::context::{ConcreteBaseAudioContext};

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum EventType {
    Ended
}

// maybe not the right, but needs to review according to type EventHandler in capacity.rs
pub(crate) struct EventHandlerInfos {
    // could be Option w/ None meaning that its a context event (cf. onSinkChange, onStateChange, etc.)
    pub node_id: u64,
    pub event_type: EventType,
    pub callback: Box<dyn Fn() + Send + Sync + 'static>,
}

// for control thread entities
pub(crate) trait EventListener {
    fn context(&self) -> &ConcreteBaseAudioContext;

    fn id(&self) -> u64;

    // callback could be a Some<&'a dyn Fn() -> ()> so that we can remove the listener too?
    fn register_event_listener(
        &self,
        event_type: EventType,
        callback: Box<dyn Fn() + Send + Sync + 'static>,
    ) {
        let handler = EventHandlerInfos {
            node_id: self.id(),
            event_type,
            callback,
        };

        self.context().register_event_handler(handler);
    }
}

#[derive(Debug)]
pub(crate) struct EventEmitterMessage {
    // could be Option w/ None meaning that its a context event
    pub node_id: u64, // we use raw u64 to
    // could be Option w/ None meaning the node is dropped on the render thread
    // and listeners can be cleared
    pub event_type: EventType,
}

// for render thread entities
// @todo - maybe should be a (bounded?) queue
pub trait EventEmitter {
    fn stagging_event(&mut self) -> &mut Option<EventType>;

    fn register_event(&mut self, event: EventType) {
        // @note: weird this is called a lot of time just as if the node was
        // never removed from the graph
        self.stagging_event().get_or_insert(event);
    }

    fn get_stagging_event(&mut self) -> Option<EventType> {
        self.stagging_event().take()
    }
}

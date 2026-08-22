use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};
use crate::state_store::app::{action::Action, store::Store};

#[allow(dead_code)]
pub struct Dispatcher {
    store: Store,
    component_reciever: UnboundedReceiver<Action>,
    controller_sender: UnboundedSender<Action>,
    controller_reciever: UnboundedReceiver<Action>,
}

impl Dispatcher {
    pub fn dispatch(&mut self, action: Action){
        self.store.update(action);
    }
}



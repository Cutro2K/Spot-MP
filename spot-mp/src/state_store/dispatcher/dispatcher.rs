use tokio::sync::mpsc::{Receiver, UnboundedReceiver, UnboundedSender};

use crate::state_store::app::{action::Action, store::Store};

#[allow(dead_code)]
pub struct Dispatcher {
    store: Store,
    component_reciever: UnboundedReceiver<Action>,
    controller_sender: UnboundedSender<Action>,
    controller_reciever: Receiver<Action>,
}

impl Dispatcher {
    fn dispatch(&mut self, action: Action){
        self.store.update(action);
    }
}



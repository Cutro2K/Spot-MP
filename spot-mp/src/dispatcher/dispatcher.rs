use std::sync::mpsc::{Receiver, Sender};
use crate::app::{action::Action, store::Store};

#[allow(dead_code)]
pub struct Dispatcher {
    store: Store,
    component_reciever: Receiver<Action>,
    controller_sender: Sender<Action>,
    controller_reciever: Receiver<Action>,
}

impl Dispatcher {
    pub fn dispatch(&mut self, action: Action){
        self.store.update(action);
    }
}



use crossbeam_channel::Sender;
use crossbeam_channel::{unbounded, Receiver};
use log::debug;
use std::fmt::Debug;
use web_time::Duration;

pub struct Binding<T, F>
where
    T: Send + 'static + Debug,
    F: Send + 'static + Debug,
{
    sender: Sender<F>,
    receiver: Receiver<T>,
}

impl<T, F> Clone for Binding<T, F>
where
    T: Send + 'static + Debug,
    F: Send + 'static + Debug,
{
    fn clone(&self) -> Self {
        Binding {
            sender: self.sender.clone(),
            receiver: self.receiver.clone(),
        }
    }
}

impl<T, F> Binding<T, F>
where
    T: Send + 'static + Debug,
    F: Send + 'static + Debug,
{
    pub fn new() -> (Self, Sender<T>, Receiver<F>) {
        let (to_sender, to_receiver) = unbounded();
        let (from_sender, from_receiver) = unbounded();
        (
            Binding {
                sender: from_sender,
                receiver: to_receiver,
            },
            to_sender,
            from_receiver,
        )
    }

    pub fn send(&self, value: F) {
        debug!("Sending message: {:?}", value);
        self.sender.send(value).unwrap();
    }

    pub fn recv(&self, timeout: Duration) -> Option<T> {
        match self.receiver.recv_timeout(timeout) {
            Ok(value) => {
                debug!("Received message: {:?}", value);
                Some(value)
            }
            Err(_) => None,
        }
    }
}

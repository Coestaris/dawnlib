use log::error;
use std::sync::mpsc::{Receiver, Sender};
use std::sync::{Condvar, Mutex};

pub struct Controller<M> {
    sender: Sender<M>,
}

pub struct ControlReceiver<M> {
    controller: Receiver<M>,
}

impl<M> Controller<M> {
    #[inline]
    pub(crate) fn send(&self, message: M) -> Result<(), String> {
        self.sender
            .send(message)
            .map_err(|e| format!("Failed to send message: {}", e))
    }
}

impl<M> ControlReceiver<M> {
    #[inline]
    pub(crate) fn receive(&self) -> Option<M> {
        self.controller.try_recv().ok()
    }
}

pub(crate) fn new_control<M>() -> (Controller<M>, ControlReceiver<M>) {
    let (sender, receiver) = std::sync::mpsc::channel();
    (
        Controller { sender },
        ControlReceiver {
            controller: receiver,
        },
    )
}

pub struct DeviceController {
    update_bus: (Mutex<u8>, Condvar),
}

impl DeviceController {
    pub fn new() -> Self {
        DeviceController {
            update_bus: (Mutex::new(0), Condvar::new()),
        }
    }

    pub fn send_and_notify<M>(&self, sender: &Controller<M>, message: M) {
        if let Err(e) = sender.send(message) {
            log::error!("Failed to send message: {}", e);
        } else {
            self.notify();
        }
    }

    pub fn send<M>(&self, sender: &Controller<M>, message: M) {
        if let Err(e) = sender.send(message) {
            log::error!("Failed to send message: {}", e);
        }
    }

    pub fn notify(&self) {
        let (lock, cvar) = &self.update_bus;
        let mut update = lock.lock().unwrap();
        *update += 1;
        cvar.notify_all();
    }

    pub(crate) fn wait_for_update(&self) {
        let (lock, cvar) = &self.update_bus;
        let mut update = lock.lock().unwrap();
        while *update == 0 {
            update = cvar.wait(update).unwrap();
        }
        *update -= 1;
    }

    pub fn reset(&self) {
        let (lock, cvar) = &self.update_bus;
        let mut update = lock.lock().unwrap();
        *update = 1;
        cvar.notify_all();
    }
}

use crate::binding::Binding;
use crate::ir::IRAsset;
use crate::requests::task::AssetTaskID;
use crate::{AssetHeader, AssetID};
use crossbeam_channel::Receiver;
use crossbeam_channel::Sender;
use web_time::Duration;

#[derive(Debug)]
pub enum ToReaderMessage {
    Enumerate(AssetTaskID),
    Read(AssetTaskID, AssetID),
}

#[derive(Debug)]
pub enum FromReaderMessage {
    Enumerate(AssetTaskID, anyhow::Result<Vec<AssetHeader>>),
    Read(AssetTaskID, AssetID, anyhow::Result<IRAsset>),
}

pub struct ReaderBinding {
    inner: Binding<ToReaderMessage, FromReaderMessage>,
}

impl ReaderBinding {
    pub(crate) fn new() -> (Self, Sender<ToReaderMessage>, Receiver<FromReaderMessage>) {
        let (inner, to_sender, from_receiver) = Binding::new();
        (ReaderBinding { inner }, to_sender, from_receiver)
    }

    pub fn send(&self, message: FromReaderMessage) {
        self.inner.send(message)
    }

    pub fn recv(&self, timeout: Duration) -> Option<ToReaderMessage> {
        self.inner.recv(timeout)
    }
}

// It uses a queue to receive load requests and another queue to send out read assets.
pub struct BasicReader {
    binding: Option<ReaderBinding>,
}

impl BasicReader {
    pub fn new() -> Self {
        BasicReader { binding: None }
    }

    fn send(&self, message: FromReaderMessage) {
        if let Some(binding) = &self.binding {
            binding.send(message);
        }
    }

    fn recv(&self, timeout: Duration) -> Option<ToReaderMessage> {
        if let Some(binding) = &self.binding {
            binding.recv(timeout)
        } else {
            None
        }
    }

    pub fn bind(&mut self, binding: ReaderBinding) {
        self.binding = Some(binding);
    }

    pub fn process_events<E, R>(&self, enumerate: E, read: R, timeout: Duration)
    where
        E: Fn() -> anyhow::Result<Vec<AssetHeader>>,
        R: Fn(AssetID) -> anyhow::Result<IRAsset>,
    {
        while let Some(msg) = self.recv(timeout) {
            match msg {
                ToReaderMessage::Enumerate(task_id) => match enumerate() {
                    Ok(headers) => {
                        self.send(FromReaderMessage::Enumerate(task_id, Ok(headers)));
                    }
                    Err(err) => {
                        self.send(FromReaderMessage::Enumerate(task_id, Err(err)));
                    }
                },
                ToReaderMessage::Read(task_id, asset_id) => match read(asset_id.clone()) {
                    Ok(asset) => {
                        self.send(FromReaderMessage::Read(task_id, asset_id, Ok(asset)));
                    }
                    Err(err) => {
                        self.send(FromReaderMessage::Read(task_id, asset_id, Err(err)));
                    }
                },
            }
        }
    }
}

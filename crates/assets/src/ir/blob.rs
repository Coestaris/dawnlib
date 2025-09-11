use serde::{Deserialize, Serialize};
use std::fmt::Debug;

#[derive(Serialize, Deserialize, Clone)]
pub struct IRBlob {
    #[serde(with = "serde_bytes")]
    pub data: Vec<u8>,
}

impl Debug for IRBlob {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("IRBlob")
            .field("data_length", &self.data.len())
            .finish()
    }
}

impl IRBlob {
    pub fn memory_usage(&self) -> usize {
        let mut sum = size_of::<IRBlob>();
        sum += self.data.capacity() * size_of::<u8>();
        sum
    }
}

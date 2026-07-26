use std::time::Instant;
use std::error::Error;

pub trait AudioSource: Send {
    fn self_test(&mut self) -> Result<(), Box<dyn Error>>;
    fn next_chunk(&mut self) -> Option<Vec<f32>>;
}

pub struct AudioFrame {
    pub timestamp: Instant,
    pub samples: Vec<f32>,
    pub dbfs: f32
}
use sadnes_core::sinks::*;

pub struct AudioFrameSink {}

impl AudioFrameSink {
    pub fn new() -> AudioFrameSink {
        AudioFrameSink {}
    }
}

impl Sink<AudioFrame> for AudioFrameSink {
    fn append(&mut self, frame: AudioFrame) {
//        unimplemented!()
    }
}

use memory::Memory;

pub struct Input {

}

impl Memory for Input {
    fn load_byte(&self, address: u16) -> u8 {
        //TODO: Implement
        0
    }

    fn store_byte(&mut self, address: u16, value: u8) {
        //TODO: Implement
    }
}

use std::mem;

pub type AudioFrame = (i16, i16);

// pub enum AudioFrame {
//     U16(u16, u16),
//     I16(i16, i16),
//     F32(f32, f32),
// }

pub struct AudioSink<'a> {
    pub buffer: &'a mut [AudioFrame],
    pub buffer_pos: usize,
}

impl<'a> AudioSink<'a> {
    pub fn append(&mut self, frame: AudioFrame) {
        self.buffer[self.buffer_pos] = frame;
        self.buffer_pos += 1;
    }
}

pub enum PixelBuffer<'a> {
    Xrgb1555(&'a mut [u16], usize),
    Rgb565(&'a mut [u16], usize),
    Xrgb8888(&'a mut [u32], usize),
}

impl<'a> PixelBuffer<'a> {
    pub fn pitch(&self) -> usize {
        match self {
            &PixelBuffer::Xrgb1555(_, pitch) => pitch,
            &PixelBuffer::Rgb565(_, pitch) => pitch,
            &PixelBuffer::Xrgb8888(_, pitch) => pitch,
        }
    }
}

pub trait VideoSink {
    fn append(&mut self, frame_buffer: &[u8]);
    fn is_populated(&self) -> bool;
    fn pixel_size(&self) -> usize;
}

pub struct Rgb565VideoSink<'a> {
    buffer: &'a mut [u16],
    is_populated: bool,
}

impl<'a> Rgb565VideoSink<'a> {
    pub fn new(buffer: &'a mut [u16]) -> Self {
        Rgb565VideoSink {
            buffer,
            is_populated: false,
        }
    }
}

impl<'a> VideoSink for Rgb565VideoSink<'a> {
    fn append(&mut self, frame_buffer: &[u8]) {
        for (i, palette_index) in frame_buffer.iter().enumerate() {
            self.buffer[i] = RGB565_PALETTE[*palette_index as usize];
        }
        self.is_populated = true;
    }

    fn is_populated(&self) -> bool {
        self.is_populated
    } 

    fn pixel_size(&self) -> usize {
        mem::size_of::<u16>()
    }
}

pub struct Xrgb1555VideoSink<'a> {
    buffer: &'a mut [u16],
    is_populated: bool,
}

impl<'a> Xrgb1555VideoSink<'a> {
    pub fn new(buffer: &'a mut [u16]) -> Self {
        Xrgb1555VideoSink {
            buffer,
            is_populated: false,
        }
    }
}

impl<'a> VideoSink for Xrgb1555VideoSink<'a> {
    fn append(&mut self, frame_buffer: &[u8]) {
        for (i, palette_index) in frame_buffer.iter().enumerate() {
            self.buffer[i] = XRGB1555_PALETTE[*palette_index as usize];
        }
        self.is_populated = true;
    }

    fn is_populated(&self) -> bool {
        self.is_populated
    } 

    fn pixel_size(&self) -> usize {
        mem::size_of::<u16>()
    }
}

pub struct Xrgb8888VideoSink<'a> {
    buffer: &'a mut [u32],
    is_populated: bool,
}

impl<'a> Xrgb8888VideoSink<'a> {
    pub fn new(buffer: &'a mut [u32]) -> Self {
        Xrgb8888VideoSink {
            buffer,
            is_populated: false,
        }
    }
}

impl<'a> VideoSink for Xrgb8888VideoSink<'a> {
    fn append(&mut self, frame_buffer: &[u8]) {
        for (i, palette_index) in frame_buffer.iter().enumerate() {
            self.buffer[i] = XRGB8888_PALETTE[*palette_index as usize];
        }
        self.is_populated = true;
    }

    fn is_populated(&self) -> bool {
        self.is_populated
    } 

    fn pixel_size(&self) -> usize {
        mem::size_of::<u32>()
    }
}

static XRGB8888_PALETTE: &[u32] = &[
    0x666666, 0x002A88, 0x1412A7, 0x3B00A4, 0x5C007E, 0x6E0040, 0x6C0600, 0x561D00,
    0x333500, 0x0B4800, 0x005200, 0x004F08, 0x00404D, 0x000000, 0x000000, 0x000000,
    0xADADAD, 0x155FD9, 0x4240FF, 0x7527FE, 0xA01ACC, 0xB71E7B, 0xB53120, 0x994E00,
    0x6B6D00, 0x388700, 0x0C9300, 0x008F32, 0x007C8D, 0x000000, 0x000000, 0x000000,
    0xFFFEFF, 0x64B0FF, 0x9290FF, 0xC676FF, 0xF36AFF, 0xFE6ECC, 0xFE8170, 0xEA9E22,
    0xBCBE00, 0x88D800, 0x5CE430, 0x45E082, 0x48CDDE, 0x4F4F4F, 0x000000, 0x000000,
    0xFFFEFF, 0xC0DFFF, 0xD3D2FF, 0xE8C8FF, 0xFBC2FF, 0xFEC4EA, 0xFECCC5, 0xF7D8A5,
    0xE4E594, 0xCFEF96, 0xBDF4AB, 0xB3F3CC, 0xB5EBF2, 0xB8B8B8, 0x000000, 0x000000,
];

lazy_static! {
    static ref XRGB1555_PALETTE: [u16; 64] = {
        let mut palette = [0; 64];
        for n in 0..64 {
            let color = XRGB8888_PALETTE[n];
            let r = ((color >> 19) & 0x1F) as u16;
            let g = ((color >> 11) & 0x1F) as u16;
            let b = ((color >> 3) & 0x1F) as u16;
            palette[n] = (r << 10) | (g << 5) | b;
        }
        palette
    };

    static ref RGB565_PALETTE: [u16; 64] = {
        let mut palette = [0; 64];
        for n in 0..64 {
            let color = XRGB8888_PALETTE[n];
            let r = ((color >> 19) & 0x1F) as u16;
            let g = ((color >> 10) & 0x3F) as u16;
            let b = ((color >> 3) & 0x1F) as u16;
            palette[n] = (r << 11) | (g << 5) | b;
        }
        palette
    };
}
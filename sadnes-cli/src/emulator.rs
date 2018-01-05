use minifb::{WindowOptions, Window, Key, KeyRepeat, Scale};
use time::precise_time_ns;

use command::*;
use audio_frame_sink::AudioFrameSink;
use video_frame_sink::VideoFrameSink;
use liner;

use sadnes_core::cartridge::{Cartridge, LoadError};
use sadnes_core::disassembler::Disassembler;
use sadnes_core::nes::Nes;
use sadnes_core::ppu::{SCREEN_WIDTH, SCREEN_HEIGHT};
use sadnes_core::sinks::*;
use sadnes_core::memory::Memory;

use std::collections::{HashSet, HashMap};
use std::env;
use std::fs::File;
use std::io::{stdin, stdout, Write};
use std::thread::{self, JoinHandle};
use std::time;
use std::sync::mpsc::{channel, Sender, Receiver};
use std::cmp::min;

const CPU_CYCLE_TIME_NS: u64 = 559;

#[derive(PartialEq, Eq)]
enum Mode {
    Running,
    Debugging,
}

pub struct Emulator {
    window: Window,

    nes: Nes,
    mode: Mode,

    breakpoints: HashSet<u16>,
    labels: HashMap<String, u16>,

    cursor: u16,
    last_command: Option<Command>,

    prompt_sender: Sender<String>,
    stdin_receiver: Receiver<String>,

    start_time_ns: u64,
    emulated_cycles: u64,
}

impl Emulator {
    pub fn new(cartridge: Cartridge) -> Emulator {
        let (prompt_sender, prompt_receiver) = channel();
        let (stdin_sender, stdin_receiver) = channel();
        let _stdin_thread = thread::spawn(move || {
            let mut con = liner::Context::new();
            loop {
                if let Ok(prompt) = prompt_receiver.recv() {
                    let res = con.read_line(prompt,
                                                &mut |_| {});
                    if let Ok(res) = res {
                        stdin_sender.send(res.as_str().into()).unwrap();
                        con.history.push(res.into()).unwrap();
                    }
                }
            }
        });

        Emulator {
            window: Window::new("sadNES",
                                SCREEN_WIDTH, SCREEN_HEIGHT,
                                WindowOptions {
                                    borderless: false,
                                    title: true,
                                    resize: false,
                                    scale: Scale::X2,
                                }
            ).unwrap(),

            nes: Nes::new(cartridge),
            mode: Mode::Running,

            breakpoints: HashSet::new(),
            labels: HashMap::new(),

            prompt_sender,
            stdin_receiver,

            cursor: 0,
            last_command: None,

            start_time_ns: 0,
            emulated_cycles: 0,
        }
    }

    pub fn run(&mut self, start_debugger: bool) {
        self.start_time_ns = precise_time_ns();

        if start_debugger {
            self.start_debugger();
        }

        let frame_buffer: Vec<u32> = vec![0; SCREEN_WIDTH * SCREEN_HEIGHT];
        let mut video_frame_sink = VideoFrameSink::new();
        let mut audio_frame_sink = AudioFrameSink::new();

        while self.window.is_open() && !self.window.is_key_down(Key::Escape) {
            self.window.update_with_buffer(&frame_buffer).unwrap();

            let target_time_ns = precise_time_ns() - self.start_time_ns;
            let target_cycles = target_time_ns / CPU_CYCLE_TIME_NS;

            match self.mode {
                Mode::Running => {
                    let mut start_debugger = false;

                    while self.emulated_cycles < target_cycles && !start_debugger {
                        let (_, trigger_watchpoint) =
                            self.step(&mut video_frame_sink, &mut audio_frame_sink);
                        if trigger_watchpoint ||
                            (self.breakpoints.len() != 0 &&
                                self.breakpoints.contains(&self.nes.cpu.regs().pc)) {
                            start_debugger = true;
                        }
                    }

                    if start_debugger {
                        self.start_debugger();
                    }
                },
                Mode::Debugging => {
                    if self.run_debugger_commands(&mut video_frame_sink, &mut audio_frame_sink) {
                        break;
                    }

                    self.window.update();
                }
            }

            if self.window.is_key_pressed(Key::F12, KeyRepeat::No) {
                self.start_debugger();
            }

            thread::sleep(time::Duration::from_millis(3));
        }
    }

    fn step(&mut self,
            video_frame_sink: &mut Sink<VideoFrame>,
            audio_frame_sink: &mut Sink<AudioFrame>) -> (u32, bool) {
        let (cycles, trigger_watchpoint) =
            self.nes.step(video_frame_sink, audio_frame_sink);

        self.emulated_cycles += cycles as u64;

        (cycles, trigger_watchpoint)
    }

    fn start_debugger(&mut self) {
        self.mode = Mode::Debugging;

        self.cursor = self.nes.cpu.regs().pc;

        print!("0x{:04x}  ", self.cursor);
        self.disassemble_instruction();

        self.print_cursor();
    }

    fn run_debugger_commands(&mut self,
                             video_frame_sink: &mut Sink<VideoFrame>,
                             audio_frame_sink: &mut Sink<AudioFrame>) -> bool {
        while let Ok(command_string) = self.stdin_receiver.try_recv() {
            let command =
                match (command_string.parse(), self.last_command.clone()) {
                    (Ok(Command::Repeat), Some(c)) => Ok(c),
                    (Ok(Command::Repeat), None) => Err("No last command".into()),
                    (Ok(c), _) => Ok(c),
                    (Err(e), _) => Err(e),
                };

            if let Ok(command) = command {
                match command {
                    Command::ShowRegs => {
                        let regs = self.nes.cpu.regs();
                        println!("pc: 0x{:04x}", regs.pc);
                        println!("a: 0x{:02x}", regs.a);
                        println!("x: 0x{:02x}", regs.x);
                        println!("y: 0x{:02x}", regs.y);
                        println!("sp: 0x{:02x}", regs.sp);
                        println!("status: 0x{:02x}", regs.status);
                        println!("Flags: {}", regs.status);
                    },
                    Command::Step(count) => {
                        for _ in 0..count {
                            self.nes.step(video_frame_sink, audio_frame_sink);
                            self.cursor = self.nes.cpu.regs().pc;
                            print!("0x{:04x}  ", self.cursor);
                            self.disassemble_instruction();
                        }
                    },
                    Command::Continue => {
                        self.mode = Mode::Running;
                        self.start_time_ns = precise_time_ns() -
                            (self.emulated_cycles * CPU_CYCLE_TIME_NS);
                    },
                    Command::Goto(address) => {
                        self.cursor = address;
                    },
                    Command::ShowMem(address) => {
                        if let Some(address) = address {
                            self.cursor = address;
                        }

                        self.print_labels_at_cursor();

                        const NUM_ROWS: u32 = 16;
                        const NUM_COLS: u32 = 16;
                        for _ in 0..NUM_ROWS {
                            print!("0x{:04x}  ", self.cursor);
                            for x in 0..NUM_COLS {
                                let byte = self.nes.interconnect.read_byte(self.cursor);
                                self.cursor = self.cursor.wrapping_add(1);
                                print!("{:02x}", byte);
                                if x < NUM_COLS - 1 {
                                    print!(" ");
                                }
                            }
                            println!();
                        }
                    },
                    Command::ShowStack => {
                        let sp = self.nes.cpu.regs().sp;
                        let addr = 0x0100 | sp as u16;

                        for i in 0..min(10, 0x01FF - addr + 1) {
                            let byte = self.nes.interconnect.read_byte(addr + i);
                            println!("0x{:04x}  {:02x}", addr + i, byte);
                        }
                    },
                    Command::Disassemble(count) => {
                        for _ in 0..count {
                            self.cursor = self.disassemble_instruction();
                        }
                    },
                    Command::Label => {
                        for (label, address) in self.labels.iter() {
                            println!(".{}: 0x{:04x}", label, address);
                        }
                    },
                    Command::AddLabel(ref label, address) => {
                        self.labels.insert(label.clone(), address);
                    },
                    Command::RemoveLabel(ref label) => {
                        if let None = self.labels.remove(label) {
                            println!("Label .{} doesn't exist", label);
                        }
                    },
                    Command::Breakpoint => {
                        for address in self.breakpoints.iter() {
                            println!("* 0x{:04x}", address);
                        }
                    },
                    Command::AddBreakpoint(address) => {
                        self.breakpoints.insert(address);
                    },
                    Command::RemoveBreakpoint(address) => {
                        if !self.breakpoints.remove(&address) {
                            println!("Breakpoint at 0x{:04x} doesn't exist", address);
                        }
                    },
                    Command::Watchpoint => {
                        for address in self.nes.cpu.watchpoints.iter() {
                            println!("* 0x{:04x}", address);
                        }
                    },
                    Command::AddWatchpoint(address) => {
                        self.nes.cpu.watchpoints.insert(address);
                    },
                    Command::RemoveWatchpoint(address) => {
                        if !self.nes.cpu.watchpoints.remove(&address) {
                            println!("Watchpoint at 0x{:04x} doesn't exist", address);
                        }
                    },
                    Command::Exit => {
                        return true;
                    },
                    Command::Repeat => unreachable!(),
                }

                self.last_command = Some(command);
            }

            if self.mode == Mode::Debugging {
                self.print_cursor();
            }
        }

        false
    }

    fn disassemble_instruction(&mut self) -> u16 {
        let mut d = Disassembler::new(self.cursor);
        println!("{}", d.disassemble_next(&mut self.nes.interconnect));
        d.pc
    }

    fn print_cursor(&self) {
        self.prompt_sender.send(format!("(sadnes-debug 0x{:04x}) > ", self.cursor)).unwrap();
    }

    fn print_labels_at_cursor(&mut self) {
        for (name, _) in self.labels.iter().filter(|x| *x.1 == self.cursor) {
            println!(".{}:", name);
        }
    }
}

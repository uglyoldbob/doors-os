//! Kernel module for x86 vga text using video mode

use crate::modules::video::TextDisplayTrait;

use crate::boot::x86::IoPortArray;
use crate::boot::x86::IoPortRef;
use crate::boot::x86::IoReadWrite;
use crate::boot::x86::IOPORTS;

/// The memory portion of the x86 hardware
pub struct X86VgaHardware {
    /// The actual memory
    buf: [u8; 0x20000],
}

/// The structure for vga hardware. This driver assumes color mode only.
pub struct X86VgaMode {
    /// The column where the next character will be placed
    column: u8,
    /// The row where the next character will be placed
    row: u8,
    /// A mutable reference to the hardware memory
    hw: &'static mut X86VgaHardware,
    /// The io ports for the vga hardware
    ports: IoPortArray<'static>,
}

/*
See https://github.com/rust-osdev/vga for information on this block comment
let mode = Graphics640x480x16::new();
    nothing interesting
mode.set_mode();
    let mut vga = VGA.lock();
        Vga::new()
    vga.set_video_mode(VideoMode::Mode320x240x256);
        vga.set_video_mode_320x240x256()
            self.set_registers(&MODE_320X240X256_CONFIGURATION);
                let emulation_mode = self.get_emulation_mode();
                self.general_registers.write_msr(configuration.miscellaneous_output);
                for (index, value) in configuration.sequencer_registers {
                    self.sequencer_registers.write(*index, *value);
                }
                self.unlock_crtc_registers(emulation_mode);
                for (index, value) in configuration.crtc_controller_registers {
                    self.crtc_controller_registers
                        .write(emulation_mode, *index, *value);
                }
                for (index, value) in configuration.graphics_controller_registers {
                    self.graphics_controller_registers.write(*index, *value);
                }
                self.attribute_controller_registers.blank_screen(emulation_mode);
                for (index, value) in configuration.attribute_controller_registers {
                    self.attribute_controller_registers
                        .write(emulation_mode, *index, *value);
                }
                self.attribute_controller_registers.unblank_screen(emulation_mode);
            self.most_recent_video_mode = Some(VideoMode::Mode320x240x256);
    // Some bios mess up the palette when switching modes,
    // so explicitly set it.
    vga.color_palette_registers.load_palette(&DEFAULT_PALETTE);
mode.clear_screen(Color16::Black);
    let frame_buffer = self.get_frame_buffer();
        usize::from(VGA.lock().get_frame_buffer()) as *mut u8
            based on bits 2-4 of the miscellaneous of the graphics control registers
    VGA.lock().sequencer_registers.set_plane_mask(PlaneMask::ALL_PLANES);
        let original_value = self.read(SequencerIndex::PlaneMask) & 0xF0;
        self.write(SequencerIndex::PlaneMask, original_value | u8::from(plane_mask),);
            self.set_index(index);
            unsafe { self.srx_data.write(value); }
    unsafe { frame_buffer.write_bytes(color, Self::SIZE); }
mode.draw_line((80, 60), (80, 420), Color16::White);
mode.draw_line((80, 60), (540, 60), Color16::White);
mode.draw_line((80, 420), (540, 420), Color16::White);
mode.draw_line((540, 420), (540, 60), Color16::White);
mode.draw_line((80, 90), (540, 90), Color16::White);
for (offset, character) in "Hello World!".chars().enumerate() {
    mode.draw_character(270 + offset * 8, 72, character, Color16::White)
}

*/

impl X86VgaMode {
    /// Gets an instance of the X86Vga. This should be protected by a singleton type pattern to prevent multiple instances from being handed out to the kernel.
    pub unsafe fn get(adr: usize) -> Option<Self> {
        let ports = IOPORTS.get_ports(0x3c0, 32).unwrap();
        let mut check = Self {
            hw: &mut *(adr as *mut X86VgaHardware),
            column: 0,
            row: 0,
            ports,
        };
        let emulation_mode = check.read_misc_output_register() & 1;
        check.write_misc_output_register(0x63);
        check.write_sequencer_register(0, 3);
        check.write_sequencer_register(1, 1);
        check.write_sequencer_register(2, 0xf);
        check.write_sequencer_register(3, 0);
        check.write_sequencer_register(4, 6);
        check.unlock_crtc_registers(emulation_mode);
        for (i, val) in [
            0x5f, 0x4f, 0x50, 0x82, 0x54, 0x80, 0x0d, 0x3e, 0x00, 0x41, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0xea, 0xac, 0xdf, 0x28, 0x00, 0xe7, 0x06, 0xe3, 0xff,
        ]
        .iter()
        .enumerate()
        {
            check.write_crt_controller_register(i as u8, *val);
        }

        for (i, val) in [0, 0, 0, 0, 0, 0x40, 0x05, 0x0f, 0xff].iter().enumerate() {
            check.write_graphics_register(i as u8, *val);
        }

        check.blank_screen();

        for (i, val) in [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 0x0a, 0x0b, 0x0c, 0x0d, 0x0e, 0x0f, 0x41, 0, 0x0f, 0x00, 0x00].iter().enumerate() {
            check.write_attribute_color_register(i as u8, *val);
        }

        for i in check.hw.buf.iter_mut() {
            *i = 42;
        }

        check.unblank_screen();

        Some(check)
    }

    fn read_misc_output_register(&self) -> u8 {
        self.ports.port(0xc).port_read()
    }

    fn write_misc_output_register(&self, v: u8) {
        self.ports.port(0x2).port_write(v);
    }

    fn read_feature_control_register(&self) -> u8 {
        self.ports.port(0xA).port_read()
    }

    fn write_feature_control_register(&mut self, v: u8) {
        self.ports.port(0x1a).port_write(v);
    }

    fn read_status0_register(&self) -> u8 {
        self.ports.port(2).port_read()
    }

    fn read_status1_register(&self) -> u8 {
        self.ports.port(0x1a).port_read()
    }

    fn read_graphics_register(&mut self, i: u8) -> u8 {
        self.ports.port(0xe).port_write(i);
        self.ports.port(0xf).port_read()
    }

    fn write_graphics_register(&mut self, i: u8, val: u8) {
        self.ports.port(0xe).port_write(i);
        self.ports.port(0xf).port_write(val);
    }

    fn read_sequencer_register(&mut self, i: u8) -> u8 {
        self.ports.port(0x4).port_write(i);
        self.ports.port(0x5).port_read()
    }

    fn write_sequencer_register(&mut self, i: u8, val: u8) {
        self.ports.port(0x4).port_write(i);
        self.ports.port(0x5).port_write(val);
    }

    fn read_attribute_color_register(&mut self, i: u8) -> u8 {
        let _ : u8 = self.ports.port(0x1a).port_read();
        self.ports.port(0x0).port_write(i);
        self.ports.port(0x1).port_read()
    }

    fn write_attribute_color_register(&mut self, i: u8, val: u8) {
        let _ : u8 = self.ports.port(0x1a).port_read();
        self.ports.port(0x0).port_write(i);
        self.ports.port(0x0).port_write(val);
    }

    fn read_crt_controller_register(&mut self, i: u8) -> u8 {
        self.ports.port(0x14).port_write(i);
        self.ports.port(0x15).port_read()
    }

    fn write_crt_controller_register(&mut self, i: u8, val: u8) {
        self.ports.port(0x14).port_write(i);
        self.ports.port(0x15).port_write(val);
    }

    fn blank_screen(&mut self) {
        let _ : u8 = self.ports.port(0x1a).port_read();
        let v : u8 = self.ports.port(0).port_read();
        self.ports.port(0).port_write(v&0xdf);
    }

    fn unblank_screen(&mut self) {
        let _ : u8 = self.ports.port(0x1a).port_read();
        let v : u8 = self.ports.port(0).port_read();
        self.ports.port(0).port_write(v|0x20);
    }

    fn unlock_crtc_registers(&mut self, mode: u8) {
        let hblank_end = self.read_crt_controller_register(3);
        self.write_crt_controller_register(3, hblank_end | 0x80);

        let vblank_end = self.read_crt_controller_register(11);
        self.write_crt_controller_register(11, vblank_end & 0x7f);
    }

    /// Detect how much memory is present on the graphics card
    pub fn detect_memory(&mut self) -> usize {
        const MULTIPLE: usize = 32768;
        let mut ramsize = 0;
        for i in (0..self.hw.buf.len()).step_by(MULTIPLE) {
            doors_macros2::kernel_print!("Checking {:x}\r\n", i);
            self.hw.buf[i] = 0;
            core::sync::atomic::compiler_fence(core::sync::atomic::Ordering::SeqCst);
            self.hw.buf[i + 1] = 1;
            core::sync::atomic::compiler_fence(core::sync::atomic::Ordering::SeqCst);
            let val = self.hw.buf[i];
            doors_macros2::kernel_print!("Val is {} at {:p}\r\n", val, &self.hw.buf[i]);
            let good = val == 0;
            if !good {
                break;
            } else {
                ramsize = i + MULTIPLE;
            }
        }
        ramsize
    }
}

impl super::TextDisplayTrait for X86VgaMode {
    fn print_char(&mut self, d: char) {}
}

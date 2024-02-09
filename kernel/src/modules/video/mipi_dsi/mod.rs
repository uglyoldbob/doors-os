//! Code for mipi-dsi hardware

use alloc::vec::Vec;

#[cfg(kernel_machine = "stm32f769i-disco")]
pub mod stm32f769;

/// The configuration parameters for enabling dsi hardware
pub struct MipiDsiConfig {
    /// The speed of the link in bits per second
    pub link_speed: u64,
    /// The number of lanes in the link
    pub num_lanes: u8,
    /// The vcid of the display to display onto
    pub vcid: u8,
}

/// A dcs packet structure that can be sent directly over mipi-dsi
pub struct DcsPacket<'a> {
    length: usize,
    header: [u8; 4],
    data: Option<&'a [u8]>,
}

/// The types of Dcs commands
#[derive(Clone, Copy)]
#[repr(u8)]
pub enum DcsCommandType {
    /// A short write command
    ShortWrite,
    /// A short write with parameter command
    ShortWriteWithParameter,
    /// A long write command
    LongWrite,
}

impl DcsCommandType {
    /// Is the command representable with a long command, true means long command, false means short command.
    pub fn is_long(&self) -> bool {
        match self {
            DcsCommandType::LongWrite => true,
            _ => false,
        }
    }
}

bitflags::bitflags! {
    /// The flags that can be used to specify behavior of a dcs command
    #[derive(Clone, Copy)]
    pub struct DcsCommandFlags: u16 {
        /// Request an ACK from the device being addressed
        const RequestAck = 0x1;
        /// Use low power mode for the command
        const Lpm = 0x2;
    }
}

/// A dcs command that can be sent over a mipi-dsi bus.
pub struct DcsCommand<'a> {
    channel: u8,
    kind: DcsCommandType,
    flags: DcsCommandFlags,
    send: &'a [u8],
    recv: Option<&'a mut [u8]>,
}

impl<'a> DcsCommand<'a> {
    /// Create a command that corresponds to a buffer write
    pub fn create_buffer_write(
        channel: u8,
        flags: DcsCommandFlags,
        data: &'a [u8],
    ) -> Result<Self, ()> {
        if data.len() == 0 {
            return Err(());
        }
        if channel > 3 {
            return Err(());
        }
        let kind = match data.len() {
            1 => DcsCommandType::ShortWrite,
            2 => DcsCommandType::ShortWriteWithParameter,
            _ => DcsCommandType::LongWrite,
        };
        Ok(Self {
            channel,
            kind,
            flags,
            send: data,
            recv: None,
        })
    }

    /// Build a dcs packet with the command
    pub fn build_packet(&self) -> Result<DcsPacket, ()> {
        let msglength = self.send.len();
        let mut header: [u8; 4] = [0; 4];
        header[0] = (self.channel << 6) | self.kind as u8;
        let data = if self.kind.is_long() {
            header[1] = (msglength & 0xFF) as u8;
            header[2] = ((msglength >> 8) & 0xFF) as u8;
            Some(self.send)
        } else {
            if msglength > 0 {
                header[1] = self.send[0]
            }
            if msglength > 1 {
                header[2] = self.send[1]
            }
            None
        };
        let length = 4 + data.map_or(0, |f| f.len());
        Ok(DcsPacket {
            header,
            length,
            data,
        })
    }
}

/// A trait that mipi-dsi panels implement.
#[enum_dispatch::enum_dispatch]
pub trait DsiPanelTrait {
    /// Runs setup commands for the panel when initializing the hardware
    fn setup(&self, dsi: &mut MipiDsiDcs);
}

/// Represents a single mipi-dsi panel
#[enum_dispatch::enum_dispatch(DsiPanelTrait)]
pub enum DsiPanel {
    /// The orisetech otm8009a dsi panel.
    OrisetechOtm8009a(OrisetechOtm8009a),
    /// A do nothing dsi panel
    DummyPanel(DummyDsiPanel),
}

/// A dummy dsi panel
pub struct DummyDsiPanel {}

impl DsiPanelTrait for DummyDsiPanel {
    fn setup(&self, _dsi: &mut MipiDsiDcs) {}
}

/// The trait involved when sending dcs commands
#[enum_dispatch::enum_dispatch]
pub trait MipiDsiDcsTrait {
    /// Dcs command that writes a buffer, not used yet.
    fn dcs_do_command<'a>(&mut self, cmd: DcsCommand<'a>) -> Result<(), ()>;
    /// A dcs write buffer command
    fn dcs_write_buffer(&mut self, channel: u8, buf: &[u8]) -> Result<(), ()> {
        let cmd = DcsCommand::create_buffer_write(channel, DcsCommandFlags::empty(), buf);
        if cmd.is_err() {
            return Err(());
        }
        self.dcs_do_command(cmd.unwrap())
    }
}

/// A struct that can send dcs commands
#[enum_dispatch::enum_dispatch(MipiDsiDcsTrait)]
pub enum MipiDsiDcs {
    /// The dcs provider for the stm32f769
    #[cfg(kernel_machine = "stm32f769i-disco")]
    Stm32f769(stm32f769::DcsProvider),
    /// A dummy provider of dcs hosting
    Dummmy(DummyDcsProvider),
}

/// A dummy provider of dcs commands that always fails
pub struct DummyDcsProvider {}

impl MipiDsiDcsTrait for DummyDcsProvider {
    fn dcs_do_command<'a>(&mut self, _cmd: DcsCommand<'a>) -> Result<(), ()> {
        Err(())
    }
}

/// The trait that all mipi dsi providers must implement
#[enum_dispatch::enum_dispatch]
pub trait MipiDsiTrait {
    /// Enable the hardware
    fn enable(
        &self,
        config: &MipiDsiConfig,
        resolution: &super::ScreenResolution,
        panel: Option<DsiPanel>,
    );
    /// Disable the hardware
    fn disable(&self);
}

/// An enumeration of all the types of gpio controllers
#[enum_dispatch::enum_dispatch(MipiDsiTrait)]
pub enum MipiDsiProvider {
    /// The reset provider for the stm32f769i-disco board.
    #[cfg(kernel_machine = "stm32f769i-disco")]
    Stm32f769(stm32f769::Module),
    /// A fake clock provider
    Dummy(DummyMipiCsi),
}

/// A fake clock provider
pub struct DummyMipiCsi {}

impl MipiDsiTrait for DummyMipiCsi {
    fn disable(&self) {}

    fn enable(
        &self,
        _config: &MipiDsiConfig,
        _resolution: &super::ScreenResolution,
        _panel: Option<DsiPanel>,
    ) {
    }
}

/// The orisetech otm8009a panel. https://www.orientdisplay.com/pdf/OTM8009A.pdf
pub struct OrisetechOtm8009a {}

impl OrisetechOtm8009a {
    /// Create a new panel
    pub fn new() -> Self {
        Self {}
    }

    /// Write a basic command to the panel
    fn write_command(&self, dsi: &mut MipiDsiDcs, cmd: u16, data: &[u8]) {
        let first = [(cmd & 0xFF) as u8];
        dsi.dcs_write_buffer(0, &first);

        let ta = [(cmd >> 8) as u8];
        let v = ta.iter();
        let v2 = data.iter();
        let v3 = v.chain(v2);
        let v: Vec<u8> = v3.map(|v| *v).collect();
        dsi.dcs_write_buffer(0, &v);
    }
}

impl DsiPanelTrait for OrisetechOtm8009a {
    fn setup(&self, dsi: &mut MipiDsiDcs) {
        self.write_command(dsi, 0xff00, &[0x80, 9, 1]);
        self.write_command(dsi, 0xff80, &[0x80, 9]);
        self.write_command(dsi, 0xc480, &[0x30]);
        {
            use crate::modules::timer::TimerTrait;
            let mut timers = crate::kernel::TIMERS.lock();
            let tp = timers.module(0);
            drop(timers);
            let mut tpl = tp.lock();
            let timer = tpl.get_timer(0).unwrap();
            drop(tpl);
            crate::modules::timer::TimerInstanceTrait::delay_ms(&timer, 10);
        }
        self.write_command(dsi, 0xc48a, &[0x40]);
        {
            use crate::modules::timer::TimerTrait;
            let mut timers = crate::kernel::TIMERS.lock();
            let tp = timers.module(0);
            drop(timers);
            let mut tpl = tp.lock();
            let timer = tpl.get_timer(0).unwrap();
            drop(tpl);
            crate::modules::timer::TimerInstanceTrait::delay_ms(&timer, 10);
        }
        self.write_command(dsi, 0xc5b1, &[0xa9]);
        self.write_command(dsi, 0xc591, &[0x34]);
        self.write_command(dsi, 0xc0b4, &[0x50]);
        self.write_command(dsi, 0xd900, &[0x4e]);
        self.write_command(dsi, 0xc181, &[0x66]); //65 hz display frequency
        self.write_command(dsi, 0xc592, &[1]);
        self.write_command(dsi, 0xc595, &[0x34]);
        self.write_command(dsi, 0xc594, &[0x33]);
        self.write_command(dsi, 0xd800, &[0x79, 0x79]);
        self.write_command(dsi, 0xc0a3, &[0x1b]);
        self.write_command(dsi, 0xc582, &[0x83]);
        self.write_command(dsi, 0xc480, &[0x83]);
        self.write_command(dsi, 0xc1a1, &[0x0e]);
        self.write_command(dsi, 0xb3a6, &[0, 1]);
        self.write_command(dsi, 0xce80, &[0x85, 1, 0, 0x84, 1, 0]);
        self.write_command(
            dsi,
            0xcea0,
            &[0x18, 4, 3, 0x39, 0, 0, 0, 0x18, 3, 3, 0x3a, 0, 0, 0],
        );
        self.write_command(
            dsi,
            0xceb0,
            &[0x18, 2, 3, 0x3b, 0, 0, 0, 0x18, 1, 3, 0x3c, 0, 0, 0],
        );
        self.write_command(dsi, 0xcfc0, &[0x1, 1, 0x20, 0x20, 0, 0, 1, 2, 0, 0]);
        self.write_command(dsi, 0xcfd0, &[0]);
        self.write_command(dsi, 0xcb80, &[0, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
        self.write_command(dsi, 0xcb90, &[0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
        self.write_command(dsi, 0xcba0, &[0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
        self.write_command(dsi, 0xcbb0, &[0, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
        self.write_command(dsi, 0xcbc0, &[0, 4, 4, 4, 4, 4, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
        self.write_command(dsi, 0xcbe0, &[0, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
        self.write_command(
            dsi,
            0xcbf0,
            &[0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff],
        );
        self.write_command(dsi, 0xcc80, &[0, 0x26, 9, 0xb, 1, 0x25, 0, 0, 0, 0]);
        self.write_command(
            dsi,
            0xcc90,
            &[0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0x26, 0xa, 0xc, 2],
        );
        self.write_command(
            dsi,
            0xcca0,
            &[0x25, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
        );
        self.write_command(dsi, 0xccb0, &[0, 0x25, 0xc, 0xa, 2, 0x26, 0, 0, 0, 0]);
        self.write_command(
            dsi,
            0xccc0,
            &[0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0x25, 0xb, 9, 1],
        );
        self.write_command(
            dsi,
            0xccd0,
            &[0x26, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
        );
        self.write_command(dsi, 0xc581, &[0x66]);
        self.write_command(dsi, 0xf5b6, &[6]);
        self.write_command(
            dsi,
            0xe100,
            &[
                0, 9, 0xf, 0xe, 7, 0x10, 0xb, 0xa, 4, 7, 0xb, 8, 0xf, 0x10, 0xa, 1,
            ],
        );
        self.write_command(
            dsi,
            0xe200,
            &[
                0, 9, 0xf, 0xe, 7, 0x10, 0xb, 0xa, 4, 7, 0xb, 8, 0xf, 0x10, 0xa, 1,
            ],
        );
        self.write_command(dsi, 0xff00, &[0xff, 0xff, 0xff]);

        //todo!("Finish display initialization commands");
    }
}

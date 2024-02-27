//! Code for mipi-dsi hardware

use alloc::vec::Vec;

use crate::{modules::gpio::GpioPinTrait, LockedArc};

use super::ScreenResolution;

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
    header: [u8; 4],
    data: Option<&'a [u8]>,
}

/// The types of dcs commands, contained within the data portion of a dcs command instead of the header.
#[derive(Clone, Copy)]
#[repr(u8)]
pub enum DcsCommandType {
    /// Do nothing
    Nop = 0,
    /// Exit sleep mode
    ExitSleep = 0x11,
    /// Turn on the display
    DisplayOn = 0x29,
    /// Set the column address
    SetColumnAddress = 0x2a,
    /// Set the page address
    SetPageAddress = 0x2b,
    /// Start writing memory?
    WriteMemoryStart = 0x2c,
    /// Set the address mode
    SetAddressMode = 0x36,
    /// Set the pixel format
    SetPixelFormat = 0x3a,
    /// Set the display brightness
    SetDisplayBrightness = 0x51,
    ///Write control display
    WriteControlDisplay = 0x53,
    /// Set power saving feature
    WritePowerSave = 0x55,
}

/// The types of Dcs commands, as found in the dcs header
#[derive(Clone, Copy)]
#[repr(u8)]
pub enum DcsHeaderCommandType {
    /// A short write command
    ShortWrite = 5,
    /// A short read command
    ShortRead = 6,
    /// A short write with parameter command
    ShortWriteWithParameter = 0x15,
    /// A long write command
    LongWrite = 0x39,
}

impl DcsHeaderCommandType {
    /// Is the command representable with a long command, true means long command, false means short command.
    pub fn is_long(&self) -> bool {
        match self {
            DcsHeaderCommandType::LongWrite => true,
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
    kind: DcsHeaderCommandType,
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
            1 => DcsHeaderCommandType::ShortWrite,
            2 => DcsHeaderCommandType::ShortWriteWithParameter,
            _ => DcsHeaderCommandType::LongWrite,
        };
        Ok(Self {
            channel,
            kind,
            flags,
            send: data,
            recv: None,
        })
    }

    /// Create a command that contains a write followed by a read
    pub fn create_write_read(
        channel: u8,
        flags: DcsCommandFlags,
        data: &'a [u8],
        dout: &'a mut [u8],
    ) -> Result<Self, ()> {
        if data.len() == 0 {
            return Err(());
        }
        if channel > 3 {
            return Err(());
        }
        let kind = DcsHeaderCommandType::ShortRead;
        Ok(Self {
            channel,
            kind,
            flags,
            send: data,
            recv: Some(dout),
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
        Ok(DcsPacket { header, data })
    }
}

/// Errors that can occur when setting up a dsi panel
pub enum PanelSetupError {
    /// Timeout communicating with the display
    Timeout,
    /// A command for setup failed
    CommandFailed(DcsCommandError),
}

impl From<DcsCommandError> for PanelSetupError {
    fn from(value: DcsCommandError) -> Self {
        PanelSetupError::CommandFailed(value)
    }
}

/// A trait that mipi-dsi panels implement.
#[enum_dispatch::enum_dispatch]
pub trait DsiPanelTrait {
    /// Runs setup commands for the panel when initializing the hardware
    fn setup(
        &self,
        dsi: &mut MipiDsiDcs,
        resolution: &ScreenResolution,
    ) -> Result<(), PanelSetupError>;
    /// Returns an array of potential resolutions for the panel
    fn get_resolutions(&self, resout: &mut Vec<ScreenResolution>);
}

/// Represents a single mipi-dsi panel
#[enum_dispatch::enum_dispatch(DsiPanelTrait)]
pub enum DsiPanel {
    /// The orisetech otm8009a dsi panel.
    OrisetechOtm8009a(LockedArc<OrisetechOtm8009a>),
    /// A do nothing dsi panel
    DummyPanel(DummyDsiPanel),
}

/// A dummy dsi panel
pub struct DummyDsiPanel {}

impl DsiPanelTrait for DummyDsiPanel {
    fn setup(
        &self,
        _dsi: &mut MipiDsiDcs,
        _resolution: &ScreenResolution,
    ) -> Result<(), PanelSetupError> {
        Ok(())
    }

    fn get_resolutions(&self, _resout: &mut Vec<ScreenResolution>) {}
}

/// The errors that can occur when transferring a dcs command on a mipi-dsi bus.
pub enum DcsCommandError {
    /// A timeout occurred.
    Timeout,
    /// An invalid command was attempted
    InvalidCommand,
    /// An invalid packet was attempted
    InvalidPacket,
}

/// The trait involved when sending dcs commands
#[enum_dispatch::enum_dispatch]
pub trait MipiDsiDcsTrait {
    /// Dcs command that writes a buffer, not used yet.
    fn dcs_do_command<'a>(&mut self, cmd: &mut DcsCommand<'a>) -> Result<(), DcsCommandError>;
    /// A dcs write buffer command
    fn dcs_write_buffer(&mut self, channel: u8, buf: &[u8]) -> Result<(), DcsCommandError> {
        let cmd = DcsCommand::create_buffer_write(channel, DcsCommandFlags::empty(), buf);
        if cmd.is_err() {
            return Err(DcsCommandError::InvalidCommand);
        }
        self.dcs_do_command(&mut cmd.unwrap())
    }
    /// A dcs read write command
    fn dcs_read_write(
        &mut self,
        channel: u8,
        buf: &[u8],
        dout: &mut [u8],
    ) -> Result<(), DcsCommandError> {
        let cmd = DcsCommand::create_write_read(channel, DcsCommandFlags::empty(), buf, dout);
        if cmd.is_err() {
            return Err(DcsCommandError::InvalidCommand);
        }
        self.dcs_do_command(&mut cmd.unwrap())
    }

    /// A dcs nop command
    fn dcs_basic_command(
        &mut self,
        channel: u8,
        cmd: DcsCommandType,
    ) -> Result<(), DcsCommandError> {
        let data = [cmd as u8];
        let cmd = DcsCommand::create_buffer_write(channel, DcsCommandFlags::empty(), &data);
        if cmd.is_err() {
            return Err(DcsCommandError::InvalidCommand);
        }
        self.dcs_do_command(&mut cmd.unwrap())
    }

    /// Set the column boundaries
    fn dcs_set_column_address(
        &mut self,
        channel: u8,
        start: u16,
        end: u16,
    ) -> Result<(), DcsCommandError> {
        let s8 = start.to_be_bytes();
        let e8 = end.to_be_bytes();
        let data = [
            DcsCommandType::SetColumnAddress as u8,
            s8[0],
            s8[1],
            e8[0],
            e8[1],
        ];
        let cmd = DcsCommand::create_buffer_write(channel, DcsCommandFlags::empty(), &data);
        if cmd.is_err() {
            return Err(DcsCommandError::InvalidCommand);
        }
        self.dcs_do_command(&mut cmd.unwrap())
    }

    ///Set the page address
    fn dcs_set_page_address(
        &mut self,
        channel: u8,
        start: u16,
        end: u16,
    ) -> Result<(), DcsCommandError> {
        let s8 = start.to_be_bytes();
        let e8 = end.to_be_bytes();
        let data = [
            DcsCommandType::SetPageAddress as u8,
            s8[0],
            s8[1],
            e8[0],
            e8[1],
        ];
        let cmd = DcsCommand::create_buffer_write(channel, DcsCommandFlags::empty(), &data);
        if cmd.is_err() {
            return Err(DcsCommandError::InvalidCommand);
        }
        self.dcs_do_command(&mut cmd.unwrap())
    }

    /// Set the pixel format
    fn dcs_set_pixel_format(&mut self, channel: u8, format: u8) -> Result<(), DcsCommandError> {
        let data = [DcsCommandType::SetPixelFormat as u8, format];
        let cmd = DcsCommand::create_buffer_write(channel, DcsCommandFlags::empty(), &data);
        if cmd.is_err() {
            return Err(DcsCommandError::InvalidCommand);
        }
        self.dcs_do_command(&mut cmd.unwrap())
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
    fn dcs_do_command<'a>(&mut self, _cmd: &mut DcsCommand<'a>) -> Result<(), DcsCommandError> {
        Ok(())
    }
}

/// The errors that can occur enabling a dsi module.
pub enum DsiEnableError {
    /// An error occurred seting up an attached panel
    PanelError(PanelSetupError),
    /// An unknown error occurred
    Unknown,
}

/// The trait that all mipi dsi providers must implement
#[enum_dispatch::enum_dispatch]
pub trait MipiDsiTrait {
    /// Enable the hardware
    fn enable(
        &self,
        config: &MipiDsiConfig,
        panel: Option<DsiPanel>,
    ) -> Result<super::Display, DsiEnableError>;
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
        _panel: Option<DsiPanel>,
    ) -> Result<super::Display, DsiEnableError> {
        Err(DsiEnableError::Unknown)
    }
}

/// The orisetech otm8009a panel. https://www.orientdisplay.com/pdf/OTM8009A.pdf
pub struct OrisetechOtm8009a {
    reset: Option<super::super::gpio::GpioPin>,
    backlight: Option<super::super::gpio::GpioPin>,
    resolution: ScreenResolution,
}

impl OrisetechOtm8009a {
    /// Create a new panel
    pub fn new(
        reset: Option<super::super::gpio::GpioPin>,
        backlight: Option<super::super::gpio::GpioPin>,
    ) -> Self {
        let resolution = crate::modules::video::ScreenResolution {
            width: 480,
            height: 800,
            hsync: 32,
            vsync: 10,
            h_b_porch: 98,
            h_f_porch: 98,
            v_b_porch: 14,
            v_f_porch: 15,
        };
        Self {
            reset,
            backlight,
            resolution,
        }
    }

    /// Write a basic command to the panel
    fn write_command(
        &self,
        dsi: &mut MipiDsiDcs,
        cmd: u16,
        data: &[u8],
    ) -> Result<(), DcsCommandError> {
        let first = [0, (cmd & 0xFF) as u8];
        dsi.dcs_write_buffer(0, &first)?;

        let ta = [(cmd >> 8) as u8];
        let v = ta.iter();
        let v2 = data.iter();
        let v3 = v.chain(v2);
        let v: Vec<u8> = v3.map(|v| *v).collect();
        dsi.dcs_write_buffer(0, &v)?;
        Ok(())
    }
}

impl DsiPanelTrait for LockedArc<OrisetechOtm8009a> {
    fn get_resolutions(&self, resout: &mut Vec<ScreenResolution>) {
        let s = self.lock();
        resout.clear();
        resout.push(s.resolution.clone());
    }

    fn setup(
        &self,
        dsi: &mut MipiDsiDcs,
        resolution: &ScreenResolution,
    ) -> Result<(), PanelSetupError> {
        let mut s = self.lock();
        if let Some(backlight) = &mut s.backlight {
            backlight.set_output();
            backlight.write_output(true);
        }

        if let Some(r) = &mut s.reset {
            r.set_output();
            r.write_output(false);

            {
                use crate::modules::timer::TimerTrait;
                let mut timers = crate::kernel::TIMERS.lock();
                let tp = timers.module(0);
                drop(timers);
                let mut tpl = tp.lock();
                let timer = tpl.get_timer(0).unwrap();
                drop(tpl);
                crate::modules::timer::TimerInstanceTrait::delay_ms(&timer, 40);
            }

            r.write_output(true);

            {
                use crate::modules::timer::TimerTrait;
                let mut timers = crate::kernel::TIMERS.lock();
                let tp = timers.module(0);
                drop(timers);
                let mut tpl = tp.lock();
                let timer = tpl.get_timer(0).unwrap();
                drop(tpl);
                crate::modules::timer::TimerInstanceTrait::delay_ms(&timer, 240);
            }
        }

        //enter command 2 mode, enable parameter shift
        s.write_command(dsi, 0xff00, &[0x80, 9, 1])?;
        //enter orise command 2 mode
        s.write_command(dsi, 0xff80, &[0x80, 9])?;
        //porch and non-display area are gnd
        s.write_command(dsi, 0xc480, &[0x30])?;
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
        //unknown
        s.write_command(dsi, 0xc48a, &[0x40])?;
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
        //power control 4 setting, sets te level, ledon level, vcom sample
        s.write_command(dsi, 0xc5b1, &[0xa9])?;
        //charge pump settings
        s.write_command(dsi, 0xc591, &[0x34])?;
        //panel driving mode
        s.write_command(dsi, 0xc0b4, &[0x50])?;
        //vcom voltage
        s.write_command(dsi, 0xd900, &[0x4e])?;
        //display frequency idle and normal mode
        s.write_command(dsi, 0xc181, &[0x55])?; //60 hz display frequency
                                                //charge pump settings
        s.write_command(dsi, 0xc592, &[1])?;
        //charge pump settings
        s.write_command(dsi, 0xc595, &[0x34])?;
        //charge pump settings
        s.write_command(dsi, 0xc594, &[0x33])?;
        //gvdd and nvgdd voltages
        s.write_command(dsi, 0xd800, &[0x79, 0x79])?;
        // sourde driver pull low timing
        s.write_command(dsi, 0xc0a3, &[0x1b])?;
        //power control setting?
        s.write_command(dsi, 0xc582, &[0x83])?;
        //?
        s.write_command(dsi, 0xc480, &[0x83])?;
        //rgb mode setting
        s.write_command(dsi, 0xc1a1, &[0x0e])?;
        //panel type (normal panel)
        s.write_command(dsi, 0xb3a6, &[0, 1])?;
        //goa vst settings
        s.write_command(dsi, 0xce80, &[0x85, 1, 0, 0x84, 1, 0])?;
        //goa clka1 settings
        s.write_command(
            dsi,
            0xcea0,
            &[0x18, 4, 3, 0x39, 0, 0, 0, 0x18, 3, 3, 0x3a, 0, 0, 0],
        )?;
        //goa clk3 settings
        s.write_command(
            dsi,
            0xceb0,
            &[0x18, 2, 3, 0x3b, 0, 0, 0, 0x18, 1, 3, 0x3c, 0, 0, 0],
        )?;
        //goa eclk settings
        s.write_command(dsi, 0xcfc0, &[0x1, 1, 0x20, 0x20, 0, 0, 1, 2, 0, 0])?;
        //goa other settings
        s.write_command(dsi, 0xcfd0, &[0])?;
        //goa pinmux settings
        s.write_command(dsi, 0xcb80, &[0, 0, 0, 0, 0, 0, 0, 0, 0, 0])?;
        //goa pad poweron settings
        s.write_command(dsi, 0xcb90, &[0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0])?;
        //goa pad poweron settings
        s.write_command(dsi, 0xcba0, &[0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0])?;
        //goa pad poweron settings
        s.write_command(dsi, 0xcbb0, &[0, 0, 0, 0, 0, 0, 0, 0, 0, 0])?;
        //goa pad poweron settings
        s.write_command(dsi, 0xcbc0, &[0, 4, 4, 4, 4, 4, 0, 0, 0, 0, 0, 0, 0, 0, 0])?;
        //goa pad poweron settings
        s.write_command(dsi, 0xcbe0, &[0, 0, 0, 0, 0, 0, 0, 0, 0, 0])?;
        //goa pad lvd settings
        s.write_command(
            dsi,
            0xcbf0,
            &[0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff],
        )?;
        //goa settings for normal scan
        s.write_command(dsi, 0xcc80, &[0, 0x26, 9, 0xb, 1, 0x25, 0, 0, 0, 0])?;
        //goa settings for normal scan
        s.write_command(
            dsi,
            0xcc90,
            &[0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0x26, 0xa, 0xc, 2],
        )?;
        //goa settings for normal scan
        s.write_command(
            dsi,
            0xcca0,
            &[0x25, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
        )?;
        //goa settings for reverse scan
        s.write_command(dsi, 0xccb0, &[0, 0x25, 0xc, 0xa, 2, 0x26, 0, 0, 0, 0])?;
        //goa settings for normal scan
        s.write_command(
            dsi,
            0xccc0,
            &[0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0x25, 0xb, 9, 1],
        )?;
        //goa settings for normal scan
        s.write_command(
            dsi,
            0xccd0,
            &[0x26, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
        )?;
        //pump1 settings
        s.write_command(dsi, 0xc581, &[0x66])?;
        //?
        s.write_command(dsi, 0xf5b6, &[6])?;
        //gamma corrections
        s.write_command(
            dsi,
            0xe100,
            &[
                0, 9, 0xf, 0xe, 7, 0x10, 0xb, 0xa, 4, 7, 0xb, 8, 0xf, 0x10, 0xa, 1,
            ],
        )?;
        //gamma corrections
        s.write_command(
            dsi,
            0xe200,
            &[
                0, 9, 0xf, 0xe, 7, 0x10, 0xb, 0xa, 4, 7, 0xb, 8, 0xf, 0x10, 0xa, 1,
            ],
        )?;
        //?
        s.write_command(dsi, 0xff00, &[0xff, 0xff, 0xff])?;

        dsi.dcs_basic_command(0, DcsCommandType::Nop)?;
        dsi.dcs_basic_command(0, DcsCommandType::ExitSleep)?;

        {
            use crate::modules::timer::TimerTrait;
            let mut timers = crate::kernel::TIMERS.lock();
            let tp = timers.module(0);
            drop(timers);
            let mut tpl = tp.lock();
            let timer = tpl.get_timer(0).unwrap();
            drop(tpl);
            crate::modules::timer::TimerInstanceTrait::delay_ms(&timer, 120);
        }

        let data = [DcsCommandType::SetAddressMode as u8, 0];
        dsi.dcs_write_buffer(0, &data)?;

        dsi.dcs_set_column_address(0, 0, resolution.width - 1)?;
        dsi.dcs_set_page_address(0, 0, resolution.height - 1)?;

        //set pixel format
        dsi.dcs_set_pixel_format(0, 0x77)?;

        let data = [DcsCommandType::WritePowerSave as u8, 0];
        dsi.dcs_write_buffer(0, &data)?;

        dsi.dcs_basic_command(0, DcsCommandType::DisplayOn)?;
        dsi.dcs_basic_command(0, DcsCommandType::Nop)?;
        dsi.dcs_basic_command(0, DcsCommandType::WriteMemoryStart)?;

        let data = [DcsCommandType::SetDisplayBrightness as u8, 240];
        dsi.dcs_write_buffer(0, &data)?;

        let data = [DcsCommandType::WriteControlDisplay as u8, 0x24];
        dsi.dcs_write_buffer(0, &data)?;

        dsi.dcs_basic_command(0, DcsCommandType::WriteControlDisplay)?;

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
        Ok(())
    }
}

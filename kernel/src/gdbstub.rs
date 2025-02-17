//! Code for operating the gdbstub

use alloc::fmt::format;
use gdbstub::target::ext::base::singlethread::SingleThreadBase;

use crate::{kernel::OwnedDevice, modules::serial::Serial};

/// A target for the gdbstub
struct DoorsTarget {}

impl gdbstub::target::Target for DoorsTarget {
    type Arch = gdbstub_arch::x86::X86_64_SSE;
    type Error = ();

    fn base_ops(&mut self) -> gdbstub::target::ext::base::BaseOps<'_, Self::Arch, Self::Error> {
        gdbstub::target::ext::base::BaseOps::SingleThread(self)
    }

    fn guard_rail_implicit_sw_breakpoints(&self) -> bool {
        true
    }
}

impl gdbstub::target::ext::breakpoints::Breakpoints for DoorsTarget {
    fn support_hw_breakpoint(
        &mut self,
    ) -> Option<gdbstub::target::ext::breakpoints::HwBreakpointOps<'_, Self>> {
        todo!()
    }

    fn support_hw_watchpoint(
        &mut self,
    ) -> Option<gdbstub::target::ext::breakpoints::HwWatchpointOps<'_, Self>> {
        todo!()
    }

    fn support_sw_breakpoint(
        &mut self,
    ) -> Option<gdbstub::target::ext::breakpoints::SwBreakpointOps<'_, Self>> {
        todo!()
    }
}

impl gdbstub::target::ext::breakpoints::HwBreakpoint for DoorsTarget {
    fn add_hw_breakpoint(
        &mut self,
        addr: <Self::Arch as gdbstub::arch::Arch>::Usize,
        kind: <Self::Arch as gdbstub::arch::Arch>::BreakpointKind,
    ) -> gdbstub::target::TargetResult<bool, Self> {
        todo!()
    }

    fn remove_hw_breakpoint(
        &mut self,
        addr: <Self::Arch as gdbstub::arch::Arch>::Usize,
        kind: <Self::Arch as gdbstub::arch::Arch>::BreakpointKind,
    ) -> gdbstub::target::TargetResult<bool, Self> {
        todo!()
    }
}

impl SingleThreadBase for DoorsTarget {
    fn read_registers(
        &mut self,
        regs: &mut <Self::Arch as gdbstub::arch::Arch>::Registers,
    ) -> gdbstub::target::TargetResult<(), Self> {
        crate::VGA.print_str(&alloc::format!("GDBSTUB READ REGISTERS {:?}\r\n", regs));
        todo!()
    }

    fn write_registers(
        &mut self,
        regs: &<Self::Arch as gdbstub::arch::Arch>::Registers,
    ) -> gdbstub::target::TargetResult<(), Self> {
        crate::VGA.print_str(&alloc::format!("GDBSTUB WRITE REGISTERS {:?}\r\n", regs));
        todo!()
    }

    fn read_addrs(
        &mut self,
        start_addr: <Self::Arch as gdbstub::arch::Arch>::Usize,
        data: &mut [u8],
    ) -> gdbstub::target::TargetResult<usize, Self> {
        crate::VGA.print_str(&alloc::format!(
            "GDBSTUB READ ADDRS {:?} size {:x}\r\n",
            start_addr,
            data.len()
        ));
        todo!()
    }

    fn write_addrs(
        &mut self,
        start_addr: <Self::Arch as gdbstub::arch::Arch>::Usize,
        data: &[u8],
    ) -> gdbstub::target::TargetResult<(), Self> {
        crate::VGA.print_str(&alloc::format!(
            "GDBSTUB WRITE ADDRS {:?} size {:x}\r\n",
            start_addr,
            data.len()
        ));
        todo!()
    }
}

doors_macros::todo_item!("Put together a pull request for gdbstub for an async connection");
impl gdbstub::conn::Connection for OwnedDevice<Serial> {
    type Error = ();
    fn flush(&mut self) -> Result<(), Self::Error> {
        use crate::modules::serial::SerialTrait;
        self.sync_flush();
        Ok(())
    }

    fn write(&mut self, byte: u8) -> Result<(), Self::Error> {
        use crate::modules::serial::SerialTrait;
        self.sync_transmit(&[byte]);
        Ok(())
    }
}

/// asnychonously run the gdb stub over a serial port
pub async fn run() {
    crate::VGA.print_str_async("Starting gdb stub\r\n").await;
    let c = crate::kernel::SERIAL.take_device(1).unwrap();
    let gdbstub = gdbstub::stub::GdbStub::new(c);
    let mut target = DoorsTarget {};
    let gdb = gdbstub.run_state_machine(&mut target);
    if let Err(e) = &gdb {
        if e.is_connection_error() {
            crate::VGA.print_str(&alloc::format!("Connection error {:?}\r\n", e));
        }
        if e.is_target_error() {
            crate::VGA.print_str(&alloc::format!("Target error {:?}\r\n", e));
        }
    }
    let mut gdb = gdb.unwrap();
    use crate::modules::serial::SerialTrait;
    use futures::StreamExt;
    loop {
        gdb = match gdb {
            gdbstub::stub::state_machine::GdbStubStateMachine::Idle(mut gdb) => {
                doors_macros::todo_item!("Do something besides unwrap here");
                let b = gdb.borrow_conn().read_stream().next().await.unwrap();
                gdb.incoming_data(&mut target, b).unwrap()
            }
            gdbstub::stub::state_machine::GdbStubStateMachine::Running(gdb) => {
                todo!();
            }
            gdbstub::stub::state_machine::GdbStubStateMachine::CtrlCInterrupt(gdb) => {
                doors_macros::todo_item!("Do something besides unwrap here");
                todo!();
            }
            gdbstub::stub::state_machine::GdbStubStateMachine::Disconnected(gdb) => break,
        };
    }
}

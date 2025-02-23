//! Code for operating the gdbstub

use core::num::NonZero;

use gdbstub::target::ext::base::{
    multithread::{MultiThreadBase, MultiThreadResume},
};

use crate::{kernel::OwnedDevice, modules::serial::Serial};

/// A target for the gdbstub
struct DoorsTarget {}

impl gdbstub::target::Target for DoorsTarget {
    type Arch = gdbstub_arch::x86::X86_64_SSE;
    type Error = alloc::string::String;

    fn base_ops(&mut self) -> gdbstub::target::ext::base::BaseOps<'_, Self::Arch, Self::Error> {
        gdbstub::target::ext::base::BaseOps::MultiThread(self)
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
        _addr: <Self::Arch as gdbstub::arch::Arch>::Usize,
        _kind: <Self::Arch as gdbstub::arch::Arch>::BreakpointKind,
    ) -> gdbstub::target::TargetResult<bool, Self> {
        todo!()
    }

    fn remove_hw_breakpoint(
        &mut self,
        _addr: <Self::Arch as gdbstub::arch::Arch>::Usize,
        _kind: <Self::Arch as gdbstub::arch::Arch>::BreakpointKind,
    ) -> gdbstub::target::TargetResult<bool, Self> {
        todo!()
    }
}

impl MultiThreadResume for DoorsTarget {
    fn resume(&mut self) -> Result<(), Self::Error> {
        Ok(())
    }

    fn clear_resume_actions(&mut self) -> Result<(), Self::Error> {
        Ok(())
    }

    fn set_resume_action_continue(
        &mut self,
        _tid: gdbstub::common::Tid,
        _signal: Option<gdbstub::common::Signal>,
    ) -> Result<(), Self::Error> {
        Ok(())
    }
}

impl MultiThreadBase for DoorsTarget {
    fn is_thread_alive(&mut self, _tid: gdbstub::common::Tid) -> Result<bool, Self::Error> {
        Ok(true)
    }

    fn list_active_threads(
        &mut self,
        thread_is_active: &mut dyn FnMut(gdbstub::common::Tid),
    ) -> Result<(), Self::Error> {
        thread_is_active(NonZero::new(1).unwrap());
        Ok(())
    }

    fn support_resume(
        &mut self,
    ) -> Option<gdbstub::target::ext::base::multithread::MultiThreadResumeOps<'_, Self>> {
        Some(self)
    }

    fn support_single_register_access(
        &mut self,
    ) -> Option<
        gdbstub::target::ext::base::single_register_access::SingleRegisterAccessOps<
            '_,
            gdbstub::common::Tid,
            Self,
        >,
    > {
        None
    }

    fn support_thread_extra_info(
        &mut self,
    ) -> Option<gdbstub::target::ext::thread_extra_info::ThreadExtraInfoOps<'_, Self>> {
        None
    }

    fn read_registers(
        &mut self,
        regs: &mut <Self::Arch as gdbstub::arch::Arch>::Registers,
        tid: gdbstub::common::Tid,
    ) -> gdbstub::target::TargetResult<(), Self> {
        crate::VGA.print_str(&alloc::format!("GDBSTUB READ REGISTERS {:?}\r\n", regs));
        regs.eflags = 43;
        regs.regs[0] = 42;
        Ok(())
    }

    fn write_registers(
        &mut self,
        regs: &<Self::Arch as gdbstub::arch::Arch>::Registers,
        tid: gdbstub::common::Tid,
    ) -> gdbstub::target::TargetResult<(), Self> {
        crate::VGA.print_str(&alloc::format!("GDBSTUB WRITE REGISTERS {:?}\r\n", regs));
        Ok(())
    }

    fn read_addrs(
        &mut self,
        start_addr: <Self::Arch as gdbstub::arch::Arch>::Usize,
        data: &mut [u8],
        tid: gdbstub::common::Tid,
    ) -> gdbstub::target::TargetResult<usize, Self> {
        crate::VGA.print_str(&alloc::format!(
            "GDBSTUB READ ADDRS {:?} size {:x}\r\n",
            start_addr,
            data.len()
        ));
        Ok(0)
    }

    fn write_addrs(
        &mut self,
        start_addr: <Self::Arch as gdbstub::arch::Arch>::Usize,
        data: &[u8],
        tid: gdbstub::common::Tid,
    ) -> gdbstub::target::TargetResult<(), Self> {
        crate::VGA.print_str(&alloc::format!(
            "GDBSTUB WRITE ADDRS {:?} size {:x}\r\n",
            start_addr,
            data.len()
        ));
        Ok(())
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
    let mut target = DoorsTarget {};
    loop {
        crate::VGA
            .print_str_async("Starting a gdbstub instance\r\n")
            .await;
        let c = crate::kernel::SERIAL.take_device(1).unwrap();
        let gdbstub = gdbstub::stub::GdbStub::new(c);
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
                    let a = gdb.incoming_data(&mut target, b);
                    if let Err(err) = &a {
                        crate::VGA
                            .print_str_async(&alloc::format!("Gdbstub error {:?}\r\n", err))
                            .await;
                        break;
                    }
                    a.unwrap()
                }
                gdbstub::stub::state_machine::GdbStubStateMachine::Running(gdb) => {
                    todo!();
                }
                gdbstub::stub::state_machine::GdbStubStateMachine::CtrlCInterrupt(gdb) => {
                    doors_macros::todo_item!("Do something besides unwrap here");
                    todo!();
                }
                gdbstub::stub::state_machine::GdbStubStateMachine::Disconnected(_gdb) => break,
            };
        }
    }
}

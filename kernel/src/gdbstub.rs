//! Code for operating the gdbstub

use core::num::NonZero;

use alloc::{boxed::Box, string::ToString};
use gdbstub::{
    conn::ConnectionExt,
    stub::MultiThreadStopReason,
    target::ext::base::multithread::{MultiThreadBase, MultiThreadResume},
};

use crate::{
    kernel::OwnedDevice,
    modules::serial::{Serial, SerialTrait},
};

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
    fn is_thread_alive(&mut self, tid: gdbstub::common::Tid) -> Result<bool, Self::Error> {
        let s = crate::scheduler::SCHEDULER.read();
        let s = s.as_ref().unwrap().sync_access();
        let task = s.lookup(tid.into());
        Ok(task.is_some())
    }

    fn list_active_threads(
        &mut self,
        thread_is_active: &mut dyn FnMut(gdbstub::common::Tid),
    ) -> Result<(), Self::Error> {
        let s = crate::scheduler::SCHEDULER.read();
        let s = s.as_ref().unwrap().sync_access();
        for (taskid, _task) in s.iter() {
            thread_is_active(NonZero::new(taskid.value()).unwrap());
        }
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
        let s = crate::scheduler::SCHEDULER.read();
        let s = s.as_ref().unwrap().sync_access();
        let task = s.lookup(tid.into());
        if let Some(task) = task {
            if let Some((context, scontext)) = task.examine_stack() {
                regs.eflags = (scontext.rflags & 0xFFFFFFFF) as u32;
                regs.segments.cs = 8;
                regs.segments.ds = 8;
                regs.segments.es = 8;
                regs.segments.fs = 8;
                regs.segments.gs = 8;
                regs.segments.ss = 16;
                regs.regs[0] = scontext.rax;
                regs.regs[1] = context.rbx;
                regs.regs[2] = scontext.rcx;
                regs.regs[3] = scontext.rdx;
                regs.regs[4] = scontext.rsi;
                regs.regs[5] = scontext.rdi;
                regs.regs[6] = scontext.rbp;
                regs.regs[7] = context.rsp;
                regs.regs[8] = scontext.r8;
                regs.regs[9] = scontext.r9;
                regs.regs[10] = scontext.r10;
                regs.regs[11] = scontext.r11;
                regs.regs[12] = scontext.r12;
                regs.regs[13] = scontext.r13;
                regs.regs[14] = scontext.r14;
                regs.regs[15] = scontext.r15;
                Ok(())
            } else {
                Err(gdbstub::target::TargetError::NonFatal)
            }
        } else {
            Err(gdbstub::target::TargetError::NonFatal)
        }
    }

    fn write_registers(
        &mut self,
        regs: &<Self::Arch as gdbstub::arch::Arch>::Registers,
        tid: gdbstub::common::Tid,
    ) -> gdbstub::target::TargetResult<(), Self> {
        Ok(())
    }

    fn read_addrs(
        &mut self,
        start_addr: <Self::Arch as gdbstub::arch::Arch>::Usize,
        data: &mut [u8],
        tid: gdbstub::common::Tid,
    ) -> gdbstub::target::TargetResult<usize, Self> {
        Ok(0)
    }

    fn write_addrs(
        &mut self,
        start_addr: <Self::Arch as gdbstub::arch::Arch>::Usize,
        data: &[u8],
        tid: gdbstub::common::Tid,
    ) -> gdbstub::target::TargetResult<(), Self> {
        Ok(())
    }
}

/// The type for implementing the gdbstub BlockingEventLoop trait
enum GdbstubBlockingEventLoop {}

impl gdbstub::stub::run_blocking::BlockingEventLoop for GdbstubBlockingEventLoop {
    type Target = DoorsTarget;
    type Connection = OwnedDevice<Serial>;

    type StopReason = MultiThreadStopReason<u64>;

    fn wait_for_stop_reason(
        target: &mut Self::Target,
        conn: &mut Self::Connection,
    ) -> Result<
        gdbstub::stub::run_blocking::Event<Self::StopReason>,
        gdbstub::stub::run_blocking::WaitForStopReasonError<
            <Self::Target as gdbstub::target::Target>::Error,
            <Self::Connection as gdbstub::conn::Connection>::Error,
        >,
    > {
        todo!();
    }

    fn on_interrupt(
        target: &mut Self::Target,
    ) -> Result<Option<Self::StopReason>, <Self::Target as gdbstub::target::Target>::Error> {
        todo!();
        Ok(Some(
            MultiThreadStopReason::Signal(gdbstub::common::Signal::SIGINT).into(),
        ))
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

impl gdbstub::conn::ConnectionExt for OwnedDevice<Serial> {
    fn peek(&mut self) -> Result<Option<u8>, Self::Error> {
        Err(())
    }

    fn read(&mut self) -> Result<u8, Self::Error> {
        Ok(self.sync_read_byte())
    }
}

/// synchonously run the gdb stub over a serial port
pub fn sync_run() {
    let mut target = DoorsTarget {};
    loop {
        let c = crate::kernel::SERIAL.take_device(1).unwrap();
        let gdbstub = gdbstub::stub::GdbStub::new(c);
        gdbstub.run_blocking::<GdbstubBlockingEventLoop>(&mut target);
    }
}

/// asynchonously run the gdb stub over a serial port
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

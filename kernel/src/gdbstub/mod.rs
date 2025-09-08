//! Code for operating the gdbstub

use core::num::NonZero;

#[cfg(any(target_arch = "x86_64", target_arch = "x86"))]
pub mod x86;

use alloc::collections::btree_map::BTreeMap;
use gdbstub::{
    stub::MultiThreadStopReason,
    target::{
        ext::{
            base::multithread::{MultiThreadBase, MultiThreadResume},
            breakpoints::SwBreakpoint,
        },
        TargetError,
    },
};

use crate::{
    kernel::OwnedDevice,
    modules::serial::{Serial, SerialTrait},
};

/// A target for the gdbstub
struct DoorsTarget {
    /// Software breakpoints
    soft_breaks: BTreeMap<usize, u8>,
}

impl DoorsTarget {
    /// Construct a new self
    pub fn new() -> Self {
        Self {
            soft_breaks: BTreeMap::new(),
        }
    }
}

impl Drop for DoorsTarget {
    fn drop(&mut self) {
        for (address, byte) in &self.soft_breaks {
            let a: &mut u8 = &mut unsafe { *(*address as *mut u8) };
            *a = *byte;
        }
    }
}

#[cfg(target_arch = "x86_64")]
impl gdbstub::target::Target for DoorsTarget {
    type Arch = x86::X86_64_SSE;
    type Error = alloc::string::String;

    fn base_ops(&mut self) -> gdbstub::target::ext::base::BaseOps<'_, Self::Arch, Self::Error> {
        gdbstub::target::ext::base::BaseOps::MultiThread(self)
    }

    fn support_breakpoints(
        &mut self,
    ) -> Option<gdbstub::target::ext::breakpoints::BreakpointsOps<'_, Self>> {
        Some(self)
    }
}

#[cfg(target_arch = "x86")]
impl gdbstub::target::Target for DoorsTarget {
    type Arch = x86::X86_SSE;
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
        Some(self)
    }

    fn support_hw_watchpoint(
        &mut self,
    ) -> Option<gdbstub::target::ext::breakpoints::HwWatchpointOps<'_, Self>> {
        None
    }

    fn support_sw_breakpoint(
        &mut self,
    ) -> Option<gdbstub::target::ext::breakpoints::SwBreakpointOps<'_, Self>> {
        Some(self)
    }
}

impl gdbstub::target::ext::breakpoints::SwBreakpoint for DoorsTarget {
    fn add_sw_breakpoint(
        &mut self,
        addr: <Self::Arch as gdbstub::arch::Arch>::Usize,
        _kind: <Self::Arch as gdbstub::arch::Arch>::BreakpointKind,
    ) -> gdbstub::target::TargetResult<bool, Self> {
        use crate::kernel::SystemTrait;
        if let Some(b_byte) = crate::SYSTEM.read().breakpoint() {
            let a: &u8 = &unsafe { *(addr as *const u8) };
            let old_byte = *a;
            unsafe { *(addr as *mut u8) = b_byte };
            self.soft_breaks.insert(addr as usize, old_byte);
            Ok(true)
        } else {
            Ok(false)
        }
    }

    fn remove_sw_breakpoint(
        &mut self,
        addr: <Self::Arch as gdbstub::arch::Arch>::Usize,
        _kind: <Self::Arch as gdbstub::arch::Arch>::BreakpointKind,
    ) -> gdbstub::target::TargetResult<bool, Self> {
        if let Some((address, instruction_byte)) = self.soft_breaks.remove_entry(&(addr as usize)) {
            let a: &mut u8 = &mut unsafe { *(address as *mut u8) };
            *a = instruction_byte;
            Ok(true)
        } else {
            Ok(false)
        }
    }
}

impl gdbstub::target::ext::breakpoints::HwBreakpoint for DoorsTarget {
    fn add_hw_breakpoint(
        &mut self,
        addr: <Self::Arch as gdbstub::arch::Arch>::Usize,
        kind: <Self::Arch as gdbstub::arch::Arch>::BreakpointKind,
    ) -> gdbstub::target::TargetResult<bool, Self> {
        self.add_sw_breakpoint(addr, kind)
    }

    fn remove_hw_breakpoint(
        &mut self,
        addr: <Self::Arch as gdbstub::arch::Arch>::Usize,
        kind: <Self::Arch as gdbstub::arch::Arch>::BreakpointKind,
    ) -> gdbstub::target::TargetResult<bool, Self> {
        self.remove_sw_breakpoint(addr, kind)
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

    #[cfg(target_arch = "x86_64")]
    fn read_registers(
        &mut self,
        regs: &mut <Self::Arch as gdbstub::arch::Arch>::Registers,
        tid: gdbstub::common::Tid,
    ) -> gdbstub::target::TargetResult<(), Self> {
        let s = crate::scheduler::SCHEDULER.read();
        let s = s.as_ref().unwrap().sync_access();
        let task = s.lookup(tid.into());
        if let Some((_taskid, task)) = task {
            if let Some((context, scontext)) = task.examine_stack() {
                regs.eflags = (scontext.rflags & 0xFFFFFFFF) as u32;
                regs.segments.cs = 8;
                regs.segments.ds = 16;
                regs.segments.es = 16;
                regs.segments.fs = 16;
                regs.segments.gs = 16;
                regs.segments.ss = 16;
                regs.regs[0] = scontext.rax;
                regs.regs[1] = scontext.rbx;
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
                regs.regs[13] = context.r13;
                regs.regs[14] = scontext.r14;
                regs.regs[15] = scontext.r15;
                regs.rip = scontext.rip;
                Ok(())
            } else {
                Err(gdbstub::target::TargetError::Errno(42))
            }
        } else {
            Err(gdbstub::target::TargetError::Errno(43))
        }
    }

    #[cfg(target_arch = "x86")]
    fn read_registers(
        &mut self,
        regs: &mut <Self::Arch as gdbstub::arch::Arch>::Registers,
        tid: gdbstub::common::Tid,
    ) -> gdbstub::target::TargetResult<(), Self> {
        let s = crate::scheduler::SCHEDULER.read();
        let s = s.as_ref().unwrap().sync_access();
        let task = s.lookup(tid.into());
        if let Some((_taskid, task)) = task {
            if let Some((context, scontext)) = task.examine_stack() {
                regs.eflags = scontext.eflags;
                regs.segments.cs = 8;
                regs.segments.ds = 8;
                regs.segments.es = 8;
                regs.segments.fs = 8;
                regs.segments.gs = 8;
                regs.segments.ss = 16;
                regs.eax = scontext.eax;
                regs.ebx = scontext.ebx;
                regs.ecx = scontext.ecx;
                regs.edx = scontext.edx;
                regs.esi = scontext.esi;
                regs.edi = scontext.edi;
                regs.ebp = scontext.ebp;
                regs.esp = context.esp;
                regs.eip = scontext.eip;
                Ok(())
            } else {
                Err(gdbstub::target::TargetError::Errno(42))
            }
        } else {
            Err(gdbstub::target::TargetError::Errno(43))
        }
    }

    fn write_registers(
        &mut self,
        regs: &<Self::Arch as gdbstub::arch::Arch>::Registers,
        tid: gdbstub::common::Tid,
    ) -> gdbstub::target::TargetResult<(), Self> {
        let s = crate::scheduler::SCHEDULER.read();
        let mut s = s.as_ref().unwrap().sync_access();
        let task = s.lookup_mut(tid.into());
        if let Some((_taskid, task)) = task {
            task.write_registers(regs)
                .map_err(|_| TargetError::NonFatal)
        } else {
            Err(TargetError::NonFatal)
        }
    }

    fn read_addrs(
        &mut self,
        start_addr: <Self::Arch as gdbstub::arch::Arch>::Usize,
        data: &mut [u8],
        _tid: gdbstub::common::Tid,
    ) -> gdbstub::target::TargetResult<usize, Self> {
        let src = unsafe { core::slice::from_raw_parts(start_addr as *const u8, data.len()) };
        data.copy_from_slice(src);
        Ok(data.len())
    }

    fn write_addrs(
        &mut self,
        start_addr: <Self::Arch as gdbstub::arch::Arch>::Usize,
        data: &[u8],
        _tid: gdbstub::common::Tid,
    ) -> gdbstub::target::TargetResult<(), Self> {
        let dst = unsafe { core::slice::from_raw_parts_mut(start_addr as *mut u8, data.len()) };
        dst.copy_from_slice(data);
        Ok(())
    }
}

/// The type for implementing the gdbstub BlockingEventLoop trait
enum GdbstubBlockingEventLoop {}

#[cfg(target_arch = "x86_64")]
impl gdbstub::stub::run_blocking::BlockingEventLoop for GdbstubBlockingEventLoop {
    type Target = DoorsTarget;
    type Connection = OwnedDevice<Serial>;

    type StopReason = MultiThreadStopReason<u64>;

    fn wait_for_stop_reason(
        _target: &mut Self::Target,
        conn: &mut Self::Connection,
    ) -> Result<
        gdbstub::stub::run_blocking::Event<Self::StopReason>,
        gdbstub::stub::run_blocking::WaitForStopReasonError<
            <Self::Target as gdbstub::target::Target>::Error,
            <Self::Connection as gdbstub::conn::Connection>::Error,
        >,
    > {
        loop {
            if let Some(b) = conn.try_read() {
                return Ok(gdbstub::stub::run_blocking::Event::IncomingData(b));
            }
            for _ in 0..1000 {
                x86_64::instructions::nop();
            }
        }
    }

    fn on_interrupt(
        _target: &mut Self::Target,
    ) -> Result<Option<Self::StopReason>, <Self::Target as gdbstub::target::Target>::Error> {
        Ok(Some(MultiThreadStopReason::Signal(
            gdbstub::common::Signal::SIGINT,
        )))
    }
}

#[cfg(target_arch = "x86")]
impl gdbstub::stub::run_blocking::BlockingEventLoop for GdbstubBlockingEventLoop {
    type Target = DoorsTarget;
    type Connection = OwnedDevice<Serial>;

    type StopReason = MultiThreadStopReason<u32>;

    fn wait_for_stop_reason(
        _target: &mut Self::Target,
        _conn: &mut Self::Connection,
    ) -> Result<
        gdbstub::stub::run_blocking::Event<Self::StopReason>,
        gdbstub::stub::run_blocking::WaitForStopReasonError<
            <Self::Target as gdbstub::target::Target>::Error,
            <Self::Connection as gdbstub::conn::Connection>::Error,
        >,
    > {
        loop {}
    }

    fn on_interrupt(
        _target: &mut Self::Target,
    ) -> Result<Option<Self::StopReason>, <Self::Target as gdbstub::target::Target>::Error> {
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
    let mut target = DoorsTarget::new();
    loop {
        if let Some(c) = crate::kernel::SERIAL.take_device(1) {
            let gdbstub = gdbstub::stub::GdbStub::new(c);
            let _ = gdbstub.run_blocking::<GdbstubBlockingEventLoop>(&mut target);
        }
        else {
            break;
        }
    }
}

/// asynchonously run the gdb stub over a serial port
#[cfg_attr(feature = "backtrace", doors_macros::framed)]
pub async fn run() {
    crate::VGA.print_str_async("Starting gdb stub\r\n").await;
    let mut target = DoorsTarget::new();
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
                gdbstub::stub::state_machine::GdbStubStateMachine::Running(_gdb) => {
                    todo!();
                }
                gdbstub::stub::state_machine::GdbStubStateMachine::CtrlCInterrupt(_gdb) => {
                    doors_macros::todo_item!("Do something besides unwrap here");
                    todo!();
                }
                gdbstub::stub::state_machine::GdbStubStateMachine::Disconnected(_gdb) => break,
            };
        }
    }
}

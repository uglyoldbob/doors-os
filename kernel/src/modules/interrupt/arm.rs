//! Arm cortex-m specific interrupt handling code. This module provides the nested vectored
//! interrupt controller and the shared interrupt entry point that all of the interrupt
//! entries in the vector table point at.

use alloc::boxed::Box;

use crate::kernel::SystemTrait;

/// A registered interrupt handler
pub type IrqHandler = Box<dyn FnMut() + Send + 'static>;

/// The number of system exceptions that live before the device irqs in the vector table
pub const EXCEPTION_BASE: usize = 16;

/// The total number of exception and irq handler slots that can be registered
pub const MAX_HANDLERS: usize = EXCEPTION_BASE + 240;

/// All of the registered exception and irq handlers, indexed by the exception number (as read
/// from ipsr at interrupt entry).
static HANDLERS: spin::Mutex<[Option<IrqHandler>; MAX_HANDLERS]> =
    spin::Mutex::new([const { None }; MAX_HANDLERS]);

/// Holds the stack pointer of the most recently taken exception, pointing at the frame pushed
/// by [`default_handler`]. The scheduler reads this value in `Context::thread_save` to capture
/// the context of the interrupted task. It must never be modified anywhere but the assembly in
/// this file and in `crate::scheduler::arm`.
#[used]
#[no_mangle]
pub static mut DOORS_IRQ_CTX: u32 = 0;

/// Register a handler for the given exception number (the raw exception number, so device
/// irqs start at [`EXCEPTION_BASE`]).
pub fn register_handler(num: usize, handler: IrqHandler) -> bool {
    let sys = crate::SYSTEM.read();
    sys.disable_interrupts();
    let result = {
        let mut handlers = HANDLERS.lock();
        if let Some(slot) = handlers.get_mut(num) {
            *slot = Some(handler);
            true
        } else {
            false
        }
    };
    sys.enable_interrupts();
    result
}

/// Remove the handler for the given exception number.
pub fn unregister_handler(num: usize) {
    let sys = crate::SYSTEM.read();
    sys.disable_interrupts();
    if let Some(slot) = HANDLERS.lock().get_mut(num) {
        *slot = None;
    }
    sys.enable_interrupts();
}

/// Register a hard fault handler that reports the faulting context on the console. Without
/// this a hard fault lands in the generic dispatch, finds no handler, and silently spins.
pub fn register_hardfault_diagnostics() {
    register_handler(
        3,
        Box::new(|| {
            // The hardware frame sits above everything the entry stub pushed, see
            // [`default_handler`] for the frame layout.
            let sp = unsafe { core::ptr::read_volatile(&DOORS_IRQ_CTX) };
            let hw = (sp + 104) as *const u32;
            let r0 = unsafe { core::ptr::read_volatile(hw) };
            let lr = unsafe { core::ptr::read_volatile(hw.add(5)) };
            let pc = unsafe { core::ptr::read_volatile(hw.add(6)) };
            let psr = unsafe { core::ptr::read_volatile(hw.add(7)) };
            let cfsr = unsafe { core::ptr::read_volatile(0xE000_ED28 as *const u32) };
            let hfsr = unsafe { core::ptr::read_volatile(0xE000_ED2C as *const u32) };
            crate::VGA.print_fixed_str(doors_macros2::fixed_string_format!(
                "\r\nHARDFAULT! pc {:x} lr {:x} psr {:x}\r\n",
                pc,
                lr,
                psr
            ));
            crate::VGA.print_fixed_str(doors_macros2::fixed_string_format!(
                "HFSR {:x} CFSR {:x} r0 {:x}\r\n",
                hfsr,
                cfsr,
                r0
            ));
            loop {
                core::hint::spin_loop();
            }
        }),
    );
}

/// The rust side of interrupt dispatch. Called from the naked [`default_handler`] entry with
/// the raw exception number read out of ipsr.
#[no_mangle]
pub extern "C" fn doors_irq_dispatch(exception: u32) {
    // The handler is looked up under the lock but called after the lock is released. The
    // scheduler's tick handler never returns (it ends in an exception return into another
    // task), so calling it while holding the lock would leave the table permanently locked.
    // The pointed at handler stays valid because the box in the table is not moved or freed
    // while an interrupt of this line is being serviced.
    let handler: Option<*mut dyn FnMut()> = {
        let mut handlers = HANDLERS.lock();
        handlers
            .get_mut(exception as usize)
            .and_then(|a| a.as_mut())
            .map(|h| &mut **h as *mut dyn FnMut())
    };
    match handler {
        Some(handler) => unsafe { (*handler)() },
        None => {
            // There is nothing safe to do for an unhandled exception or interrupt, returning
            // would just re-enter it or continue with undefined state.
            loop {
                core::hint::spin_loop();
            }
        }
    }
}

/// The shared entry point for every handler in the vector table. It records the location of
/// the interrupted context so the scheduler can save and restore tasks, then dispatches to
/// any registered rust handler.
///
/// The frame it pushes (s16-s31, then r4-r11 and lr, then a duplicate of lr to keep the stack
/// 8 byte aligned) must match the pops performed in `crate::scheduler::arm::thread_restore`.
#[unsafe(naked)]
pub extern "C" fn default_handler() {
    core::arch::naked_asm!(
        "\
        vpush {{s16-s31}}
        push {{r4-r11, lr}}
        push {{lr}}
        mov r1, sp
        ldr r0, =DOORS_IRQ_CTX
        str r1, [r0]
        mrs r0, ipsr
        bl doors_irq_dispatch
        pop {{r1}}
        pop {{r4-r11, lr}}
        vpop {{s16-s31}}
        bx lr"
    );
}

/// The nested vectored interrupt controller of the cortex-m core.
pub struct Nvic {}

impl Nvic {
    /// Create a new reference to the nvic
    pub const fn new() -> Self {
        Self {}
    }

    /// Set the enable bit for the given irq in the nvic
    fn set_enable(irq: u8) {
        unsafe {
            let iser = (0xe000_e100 as *mut u32).add((irq >> 5) as usize);
            core::ptr::write_volatile(iser, 1u32 << (irq & 31));
        }
    }

    /// Clear the enable bit for the given irq in the nvic
    fn clear_enable(irq: u8) {
        unsafe {
            let icer = (0xe000_e180 as *mut u32).add((irq >> 5) as usize);
            core::ptr::write_volatile(icer, 1u32 << (irq & 31));
        }
    }

    /// Reads the enable bit for the given irq in the nvic
    fn is_enabled(irq: u8) -> bool {
        unsafe {
            let iser = (0xe000_e100 as *const u32).add((irq >> 5) as usize);
            (core::ptr::read_volatile(iser) & (1u32 << (irq & 31))) != 0
        }
    }
}

impl Default for Nvic {
    fn default() -> Self {
        Self::new()
    }
}

impl super::InterruptControllerTrait for Nvic {
    fn end_of_interrupt(&self, _num: u8) {
        // Exceptions on cortex-m are retired with the exception return itself, there is no
        // end of interrupt signal that needs to be sent.
    }

    fn enable_irq_sync(&self, num: u8) {
        Self::set_enable(num);
    }

    fn disable_irq_sync(&self, num: u8) {
        Self::clear_enable(num);
    }

    fn enable_irq_interrupt(&self, num: u8) {
        Self::set_enable(num);
    }

    fn disable_irq_interrupt(&self, num: u8) {
        Self::clear_enable(num);
    }

    fn is_irq_enabled(&self, irq: u8) -> bool {
        Self::is_enabled(irq)
    }

    fn lookup_irq_with_channel(&self, channel: u8) -> Option<u8> {
        // The nvic does not have any concept of routed channels
        Some(channel)
    }
}

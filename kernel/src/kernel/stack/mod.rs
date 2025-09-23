//! Contains code for unwinding the stack when there is a panic in the kernel
//!
//! This module provides DWARF-based stack unwinding capabilities using the gimli crate
//! to parse debug information and provide meaningful stack traces.

use gimli::BaseAddresses;

/// External symbols from the linker script that define the debug sections
extern "C" {
    static __debug_frame_start: u8;
    static __debug_frame_end: u8;
    static __eh_frame_start: u8;
    static __eh_frame_end: u8;
    static __debug_info_start: u8;
    static __debug_info_end: u8;
    static __debug_abbrev_start: u8;
    static __debug_abbrev_end: u8;
    static __debug_str_start: u8;
    static __debug_str_end: u8;
    static __debug_line_start: u8;
    static __debug_line_end: u8;
    static START_OF_KERNEL: u8;
    static END_OF_KERNEL: u8;
}

/// Maximum number of stack frames to unwind
const MAX_STACK_FRAMES: usize = 32;

/// A single stack frame in the unwinding process
#[derive(Debug, Clone)]
pub struct StackFrame {
    /// The instruction pointer (return address)
    pub ip: usize,
    /// The stack pointer for this frame
    pub sp: usize,
    /// The frame pointer for this frame
    pub fp: usize,
    /// Optional symbol name if resolution is available
    pub symbol: Option<&'static str>,
    /// File name if debug info is available
    pub file: Option<&'static str>,
    /// Line number if debug info is available
    pub line: Option<u32>,
}

/// Stack unwinder that uses DWARF debug information
pub struct StackUnwinder {
    /// Base addresses for DWARF sections
    bases: BaseAddresses,
    /// Whether the unwinder is initialized
    initialized: bool,
}

impl StackUnwinder {
    /// Create a new stack unwinder
    pub fn new() -> Self {
        Self {
            bases: BaseAddresses::default(),
            initialized: false,
        }
    }

    /// Initialize the unwinder with kernel debug information
    pub fn initialize(&mut self) -> Result<(), &'static str> {
        if self.initialized {
            return Ok(());
        }

        // Set up base addresses for the kernel
        unsafe {
            let kernel_start = &START_OF_KERNEL as *const u8 as usize;
            let kernel_end = &END_OF_KERNEL as *const u8 as usize;

            if kernel_end <= kernel_start {
                return Err("Invalid kernel memory layout");
            }

            self.bases = BaseAddresses::default()
                .set_text(kernel_start as u64)
                .set_got(kernel_start as u64);
        }

        self.initialized = true;
        Ok(())
    }

    /// Unwind the stack starting from the current context
    pub fn unwind_stack(&self) -> Result<alloc::vec::Vec<StackFrame>, &'static str> {
        if !self.initialized {
            return Err("Unwinder not initialized");
        }

        let mut frames = alloc::vec::Vec::with_capacity(MAX_STACK_FRAMES);

        // Get the current frame pointer and instruction pointer
        let mut current_fp: usize;
        let current_ip: usize;

        unsafe {
            core::arch::asm!(
                "mov {fp}, rbp",
                "lea {ip}, [rip]",
                fp = out(reg) current_fp,
                ip = out(reg) current_ip,
            );
        }

        frames.push(StackFrame {
            ip: current_ip,
            sp: 0, // Will be filled in if needed
            fp: current_fp,
            symbol: None,
            file: None,
            line: None,
        });

        // Walk the stack using frame pointers
        let mut frame_count = 1;
        while frame_count < MAX_STACK_FRAMES && current_fp != 0 {
            if !self.is_valid_frame_pointer(current_fp) {
                break;
            }

            unsafe {
                // Read the previous frame pointer and return address
                let frame_ptr = current_fp as *const usize;

                // Verify we can safely read from this address
                if !self.is_valid_kernel_address(frame_ptr as usize)
                    || !self.is_valid_kernel_address(frame_ptr.add(1) as usize)
                {
                    break;
                }

                let prev_fp = frame_ptr.read_volatile();
                let return_addr = frame_ptr.add(1).read_volatile();

                // Sanity check: frame pointer should increase up the stack
                if prev_fp != 0 && prev_fp <= current_fp {
                    break;
                }

                // Verify the return address is within kernel space
                if !self.is_valid_kernel_address(return_addr) {
                    break;
                }

                frames.push(StackFrame {
                    ip: return_addr,
                    sp: current_fp + 16, // Approximate stack pointer
                    fp: prev_fp,
                    symbol: self.resolve_symbol(return_addr),
                    file: None, // TODO: Add DWARF line info parsing
                    line: None, // TODO: Add DWARF line info parsing
                });

                current_fp = prev_fp;
                frame_count += 1;
            }
        }

        Ok(frames)
    }

    /// Check if a frame pointer looks valid
    fn is_valid_frame_pointer(&self, fp: usize) -> bool {
        // Frame pointer should be:
        // 1. Non-zero
        // 2. Aligned to 8 bytes (on x86_64)
        // 3. Within kernel memory space
        // 4. Not in the first 1MB (boot/low memory area)
        fp != 0
            && fp & 0x7 == 0
            && fp >= 0x100000
            && self.is_valid_kernel_address(fp)
            && self.is_valid_kernel_address(fp + 8)
    }

    /// Check if an address is within valid kernel memory
    fn is_valid_kernel_address(&self, addr: usize) -> bool {
        unsafe {
            let kernel_start = &START_OF_KERNEL as *const u8 as usize;
            let kernel_end = &END_OF_KERNEL as *const u8 as usize;

            // Basic kernel space check
            addr >= kernel_start && addr < kernel_end
        }
    }

    /// Attempt to resolve a symbol name for the given address
    fn resolve_symbol(&self, _addr: usize) -> Option<&'static str> {
        // TODO: Implement symbol resolution using DWARF debug_info
        // This would require parsing the debug_info section to find
        // function symbols and their address ranges
        None
    }

    /// Get debug section data safely
    fn get_debug_section(
        &self,
        start: *const u8,
        end: *const u8,
    ) -> Result<&'static [u8], &'static str> {
        unsafe {
            let start_addr = start as usize;
            let end_addr = end as usize;

            if end_addr <= start_addr {
                return Err("Invalid debug section bounds");
            }

            let len = end_addr - start_addr;
            if len > 0x10000000 {
                // Sanity check: 256MB max
                return Err("Debug section too large");
            }

            Ok(core::slice::from_raw_parts(start, len))
        }
    }

    /// Print a formatted stack trace
    pub fn print_stack_trace(&self) {
        crate::VGA.print_str("=== DWARF Stack Trace ===\r\n");

        match self.unwind_stack() {
            Ok(frames) => {
                for (i, frame) in frames.iter().enumerate() {
                    crate::VGA.print_fixed_str(doors_macros2::fixed_string_format!(
                        "{:2}: 0x{:016x}",
                        i,
                        frame.ip
                    ));

                    if let Some(symbol) = frame.symbol {
                        crate::VGA.print_str(" ");
                        crate::VGA.print_str(symbol);
                    }

                    if let (Some(file), Some(line)) = (frame.file, frame.line) {
                        crate::VGA.print_fixed_str(doors_macros2::fixed_string_format!(
                            " at {}:{}",
                            file,
                            line
                        ));
                    }

                    crate::VGA.print_str("\r\n");
                }
            }
            Err(e) => {
                crate::VGA.print_str("Failed to unwind stack: ");
                crate::VGA.print_str(e);
                crate::VGA.print_str("\r\n");

                // Fallback to simple frame pointer walking
                self.print_simple_stack_trace();
            }
        }
    }

    /// Simple stack trace using only frame pointers (fallback)
    fn print_simple_stack_trace(&self) {
        crate::VGA.print_str("=== Simple Stack Trace ===\r\n");

        let mut frame_ptr: usize;
        unsafe {
            core::arch::asm!("mov {}, rbp", out(reg) frame_ptr);
        }

        let mut frame_count = 0;
        while frame_count < MAX_STACK_FRAMES && frame_ptr != 0 {
            if !self.is_valid_frame_pointer(frame_ptr) {
                break;
            }

            unsafe {
                let return_addr_ptr = (frame_ptr + 8) as *const usize;
                if self.is_valid_kernel_address(return_addr_ptr as usize) {
                    let return_addr = return_addr_ptr.read_volatile();

                    crate::VGA.print_fixed_str(doors_macros2::fixed_string_format!(
                        "{:2}: 0x{:016x}\r\n",
                        frame_count,
                        return_addr
                    ));

                    let next_frame_ptr = (frame_ptr as *const usize).read_volatile();
                    if next_frame_ptr <= frame_ptr {
                        break;
                    }
                    frame_ptr = next_frame_ptr;
                } else {
                    break;
                }
            }

            frame_count += 1;
        }
    }
}

/// Global stack unwinder instance
static mut STACK_UNWINDER: Option<StackUnwinder> = None;
static UNWINDER_INIT: spin::Once<()> = spin::Once::new();

/// Initialize the global stack unwinder
pub fn init_unwinder() -> Result<(), &'static str> {
    UNWINDER_INIT.call_once(|| unsafe {
        let mut unwinder = StackUnwinder::new();
        if let Err(e) = unwinder.initialize() {
            crate::VGA.print_str("Failed to initialize stack unwinder: ");
            crate::VGA.print_str(e);
            crate::VGA.print_str("\r\n");
        }
        STACK_UNWINDER = Some(unwinder);
    });
    Ok(())
}

/// Print a stack trace using the global unwinder
pub fn print_stack_trace() {
    unsafe {
        if let Some(ref unwinder) = STACK_UNWINDER {
            unwinder.print_stack_trace();
        } else {
            crate::VGA.print_str("Stack unwinder not initialized\r\n");
        }
    }
}

/// Get the current stack frames
pub fn get_stack_frames() -> Result<alloc::vec::Vec<StackFrame>, &'static str> {
    unsafe {
        if let Some(ref unwinder) = STACK_UNWINDER {
            unwinder.unwind_stack()
        } else {
            Err("Stack unwinder not initialized")
        }
    }
}

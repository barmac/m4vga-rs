//! Classic XOR color pattern with smooth scrolling.

#![no_std]
#![no_main]

#[cfg(feature = "panic-halt")]
extern crate panic_halt;
#[cfg(feature = "panic-itm")]
extern crate panic_itm;

use core::sync::atomic::{AtomicUsize, Ordering};
use stm32f4;
use stm32f4::stm32f407::interrupt;

extern "C" {
    /// The assembly-language pattern generator found in `pattern.S`.
    fn xor_pattern_impl(
        line_number: usize,
        col_number: usize,
        target: *mut u8,
        target_size: usize,
    );
}

/// A thin Rust wrapper for the assembly routine.
fn xor_pattern(line_number: usize, col_number: usize, target: &mut [u8]) {
    // The asm routine only writes within bounds if given an even multiple of
    // four pixels. Round down to ensure this.
    let length = target.len() & !3;

    // Safety: with length rounded down, the asm routine behaves.
    unsafe {
        xor_pattern_impl(line_number, col_number, target.as_mut_ptr(), length)
    }
}

/// Demo entry point. Responsible for starting up the display driver and
/// providing callbacks.
#[allow(unused_parens)] // TODO bug in cortex_m_rt
#[cortex_m_rt::entry]
fn main() -> ! {
    let mut vga =
        m4vga::take_hardware().configure_timing(&m4vga::timing::SVGA_800_600);

    // Okay, demo time. This demo keeps a single piece of state: a frame
    // counter. We'll stack-allocate it because we can.
    let frame = AtomicUsize::new(0);

    // Now we'll start drawing and share state between the ISRs and thread.
    vga.with_raster(
        |line, tgt, ctx, _| {
            let f = frame.load(Ordering::Relaxed);
            xor_pattern(
                (line >> 2) + f, // >>2 because the pattern is upscaled 4x
                f,
                &mut tgt[0..800],
            );
            ctx.target_range = 0..800; // 800 pixels now valid
        },
        // Run a per-frame loop updating the frame counter.
        |vga| loop {
            vga.sync_to_vblank();
            frame.fetch_add(1, Ordering::Relaxed);

            // Enable outputs. This is technically wasted effort after the first
            // frame, but it costs us little, so.
            vga.video_on();
        },
    )
}

/// Wires up the PendSV handler expected by the driver.
#[cortex_m_rt::exception]
#[link_section = ".ramcode"]
fn PendSV() {
    m4vga::pendsv_raster_isr()
}

/// Wires up the TIM3 handler expected by the driver.
#[interrupt]
#[link_section = ".ramcode"]
fn TIM3() {
    m4vga::tim3_shock_isr()
}

/// Wires up the TIM4 handler expected by the driver.
#[interrupt]
#[link_section = ".ramcode"]
fn TIM4() {
    m4vga::tim4_horiz_isr()
}

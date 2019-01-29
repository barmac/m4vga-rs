//! Classic XOR color pattern with smooth scrolling.

#![no_std]
#![no_main]

// Demo mains must elect one panic-handler crate. Here we use the ITM one, which
// is low-overhead.
extern crate panic_itm;

use core::sync::atomic::{Ordering, AtomicUsize};
use stm32f4;
use stm32f4::stm32f407 as device;
use stm32f4::stm32f407::interrupt;
use m4vga_rs::vga;

extern {
    /// The assembly-language pattern generator found in `pattern.S`.
    fn xor_pattern_impl(line_number: usize,
                        col_number: usize,
                        target: *mut u8,
                        target_size: usize);
}

/// A thin Rust wrapper for the assembly routine.
fn xor_pattern(line_number: usize,
               col_number: usize,
               target: &mut [u8]) {
    // The asm routine only writes within bounds if given an even multiple of
    // four pixels. Round down to ensure this.
    let length = target.len() & !3;
    unsafe {
        xor_pattern_impl(
            line_number,
            col_number,
            target.as_mut_ptr(),
            length,
        )
    }
}

/// Demo entry point. Responsible for starting up the display driver and
/// providing callbacks.
#[allow(unused_parens)] // TODO bug in cortex_m_rt
#[cortex_m_rt::entry]
fn main() -> ! {
    // Claim exclusive control of all peripherals from the runtime. This makes
    // us responsible for divvying them up between clients; we just hand the
    // relevant ones to the display driver, below.
    let mut cp = cortex_m::peripheral::Peripherals::take().unwrap();
    let p = device::Peripherals::take().unwrap();

    // Give the driver its hardware resources...
    let mut vga = vga::init(
        cp.NVIC,
        &mut cp.SCB,
        p.FLASH,
        &p.DBG,
        p.RCC,
        p.GPIOB,
        p.GPIOE,
        p.TIM1,
        p.TIM3,
        p.TIM4,
        p.DMA2)
        // ...and select a display timing.
        .configure_timing(&m4vga_rs::vga::timing::SVGA_800_600);

    // Okay, demo time. This demo keeps a single piece of state: a frame
    // counter. We'll stack-allocate it because we can.
    let frame = AtomicUsize::new(0);

    // Now we'll start drawing and share state between the ISRs and thread.
    vga.with_raster(
        |line, tgt, ctx| {
            let f = frame.load(Ordering::Relaxed);
            xor_pattern(
                (line >> 2) + f, // >>2 because the pattern is upscaled 4x
                f,
                &mut tgt[0..800],
                );
            ctx.target_range = 0..800;  // 800 pixels now valid
        },
        // Run a per-frame loop updating the frame counter.
        |vga| loop {
            vga.sync_to_vblank();
            frame.fetch_add(1, Ordering::Relaxed);

            // Enable outputs. This is technically wasted effort after the first
            // frame, but it costs us little, so.
            vga.video_on();
        })
}

/// Wires up the PendSV handler expected by the driver.
#[cortex_m_rt::exception]
fn PendSV() {
    m4vga_rs::vga::bg_rast::maintain_raster_isr()
}

/// Wires up the TIM3 handler expected by the driver.
#[interrupt]
fn TIM3() {
    m4vga_rs::vga::shock::shock_absorber_isr()
}

/// Wires up the TIM4 handler expected by the driver.
#[interrupt]
fn TIM4() {
    m4vga_rs::vga::hstate::hstate_isr()
}

#![no_main]
#![no_std]

use blackpill_exp as _; // global logger + panicking-behavior + memory layout
use rtic;

#[rtic::app(device = stm32f4xx_hal::stm32)]
mod app {
    #[init]
    fn init(_ctx: init::Context) -> (init::LateResources, init::Monotonics) {
        defmt::info!("Hello, RTIC!");

        (init::LateResources {}, init::Monotonics())
    }

    #[idle]
    fn idle(_ctx: idle::Context) -> ! {
        loop {}
    }
}

#![no_main]
#![no_std]

use blackpill_exp as _; // global logger + panicking-behavior + memory layout
use rtic;

#[rtic::app(device = stm32f4xx_hal::pac, dispatchers = [SPI1])]
mod app {
    use dwt_systick_monotonic::DwtSystick;
    use rtic::time::duration::*;
    use stm32f4xx_hal::{
        gpio::{gpioc, Output, PushPull},
        prelude::*,
        rcc, stm32 as pac,
    };

    const SYSCLK_HZ: u32 = 100_000_000;

    #[monotonic(binds = SysTick, default = true)]
    type SysMono = DwtSystick<{ crate::app::SYSCLK_HZ }>;

    type StatusLED = gpioc::PC13<Output<PushPull>>;

    #[resources]
    struct Resources {
        led: crate::app::StatusLED,
    }

    #[init]
    fn init(ctx: init::Context) -> (init::LateResources, init::Monotonics) {
        let dp: pac::Peripherals = ctx.device;
        let rcc = dp.RCC.constrain();
        let mut dcb = ctx.core.DCB;

        // Clock setup.
        let clocks: rcc::Clocks = rcc.cfgr.use_hse(25.mhz()).sysclk(SYSCLK_HZ.hz()).freeze();
        let dwt = ctx.core.DWT;
        let systick = ctx.core.SYST;
        let mono = DwtSystick::new(&mut dcb, dwt, systick, clocks.sysclk().0);

        // LED setup.
        let gpioc = dp.GPIOC.split();
        let led: StatusLED = gpioc.pc13.into_push_pull_output();

        blink_led::spawn().unwrap();

        defmt::info!("Hello from RTIC!");

        (init::LateResources { led }, init::Monotonics(mono))
    }

    #[task(resources = [led])]
    fn blink_led(mut ctx: blink_led::Context) {
        defmt::info!("Blink!");
        ctx.resources.led.lock(|led| led.toggle().unwrap());

        blink_led::spawn_after(Seconds(1_u32)).ok();
    }

    #[idle]
    fn idle(_ctx: idle::Context) -> ! {
        loop {}
    }
}

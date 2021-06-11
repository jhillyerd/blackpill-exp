#![no_main]
#![no_std]

use blackpill_exp as _; // global logger + panicking-behavior + memory layout
use rtic;

#[rtic::app(device = stm32f4xx_hal::pac, dispatchers = [SPI1])]
mod app {
    use core::convert::TryInto;
    use dwt_systick_monotonic::DwtSystick;
    use infrared::{remotecontrol::RemoteControl, remotes::nec::Apple2009};
    use rtic::{
        rtic_monotonic::Microseconds,
        time::{duration::Milliseconds, Instant},
    };
    use stm32f4xx_hal::{gpio::*, prelude::*, rcc, stm32 as pac};

    const SYSCLK_HZ: u32 = 100_000_000;

    #[monotonic(binds = SysTick, default = true)]
    type SysMono = DwtSystick<{ crate::app::SYSCLK_HZ }>;

    type StatusLED = gpioc::PC13<Output<PushPull>>;
    type IRInput = gpiob::PB6<Input<Floating>>;
    type IRRecv = infrared::hal::EventReceiver<infrared::protocols::NecApple, IRInput>;

    #[resources]
    struct Resources {
        led: StatusLED,
        ir_recv: IRRecv,
    }

    #[init]
    fn init(ctx: init::Context) -> (init::LateResources, init::Monotonics) {
        let mut dcb = ctx.core.DCB;
        let mut dp: pac::Peripherals = ctx.device;
        let mut syscfg = dp.SYSCFG.constrain();
        let rcc = dp.RCC.constrain();
        let gpiob = dp.GPIOB.split();
        let gpioc = dp.GPIOC.split();

        // Clock setup.
        let clocks: rcc::Clocks = rcc.cfgr.use_hse(25.mhz()).sysclk(SYSCLK_HZ.hz()).freeze();
        let systick = ctx.core.SYST;
        let mono = DwtSystick::new(&mut dcb, ctx.core.DWT, systick, clocks.sysclk().0);

        // LED setup.
        let led: StatusLED = gpioc.pc13.into_push_pull_output();

        // Setup IR input.
        let mut ir_input = gpiob.pb6.into_floating_input();
        ir_input.make_interrupt_source(&mut syscfg);
        ir_input.trigger_on_edge(&mut dp.EXTI, Edge::RISING_FALLING);
        ir_input.enable_interrupt(&mut dp.EXTI);

        // Setup IR receiver, indicating we will report deltas in microseconds/10.
        let ir_recv = infrared::hal::EventReceiver::new(ir_input, 100_000);

        defmt::info!("Hello from RTIC!");

        clear_led::spawn().ok();

        (init::LateResources { led, ir_recv }, init::Monotonics(mono))
    }

    #[task(binds = EXTI9_5, priority = 2, resources = [ir_recv])]
    fn poll_ir_input(mut ctx: poll_ir_input::Context) {
        static mut PREVIOUS: Option<Instant<SysMono>> = None;

        // Determine elapsed time since last IRInput interrupt.
        let now = monotonics::SysMono::now();
        let elapsed: Option<Microseconds> = if let Some(previous) = PREVIOUS {
            now.checked_duration_since(&previous)
                .and_then(|d| d.try_into().ok())
        } else {
            None
        };
        let elapsed_us = if let Some(Microseconds(us)) = elapsed {
            us
        } else {
            0
        };
        *PREVIOUS = Some(now);

        ctx.resources.ir_recv.lock(|ir_recv: &mut IRRecv| {
            ir_recv.pin.clear_interrupt_pending_bit();
            if let Ok(Some(cmd)) = ir_recv.edge_event(elapsed_us / 10) {
                if let Some(button) = Apple2009::decode(cmd) {
                    defmt::info!("Button {} pressed", defmt::Debug2Format(&button));
                }

                // Blink LED to indicate IR received.
                blink_led::spawn().ok();
            }
        });
    }

    #[task(resources = [led])]
    fn blink_led(mut ctx: blink_led::Context) {
        static mut CLEAR: Option<clear_led::SpawnHandle> = None;

        // Cancel pending clear_led.
        if let Some(handle) = CLEAR.take() {
            handle.cancel().ok();
        }

        ctx.resources.led.lock(|led| led.set_low().ok());

        *CLEAR = clear_led::spawn_after(Milliseconds(100_u32)).ok();
    }

    #[task(resources = [led])]
    fn clear_led(mut ctx: clear_led::Context) {
        ctx.resources.led.lock(|led| led.set_high().ok());
    }

    #[idle]
    fn idle(_ctx: idle::Context) -> ! {
        loop {}
    }
}

#![no_main]
#![no_std]

use blackpill_exp as _; // global logger + panicking-behavior + memory layout
use rtic;

#[rtic::app(device = stm32f4xx_hal::pac, dispatchers = [SPI1])]
mod app {
    use dwt_systick_monotonic::DwtSystick;
    use infrared::{remotecontrol::RemoteControl, remotes::nec::Apple2009};
    use stm32f4xx_hal::{gpio::*, prelude::*, rcc, stm32 as pac, timer};

    const SYSCLK_HZ: u32 = 100_000_000;
    const IR_SAMPLE_HZ: u32 = 20_000;

    #[monotonic(binds = SysTick, default = true)]
    type SysMono = DwtSystick<{ crate::app::SYSCLK_HZ }>;

    type StatusLED = gpioc::PC13<Output<PushPull>>;
    type IRTimer = timer::Timer<pac::TIM4>;
    type IRRecv =
        infrared::hal::PeriodicReceiver<infrared::protocols::NecApple, gpiob::PB6<Input<Floating>>>;

    #[resources]
    struct Resources {
        led: StatusLED,
        ir_recv: IRRecv,
        ir_timer: IRTimer,
    }

    #[init]
    fn init(ctx: init::Context) -> (init::LateResources, init::Monotonics) {
        let mut dcb = ctx.core.DCB;

        let dp: pac::Peripherals = ctx.device;
        let rcc = dp.RCC.constrain();
        let gpiob = dp.GPIOB.split();
        let gpioc = dp.GPIOC.split();

        // Clock setup.
        let clocks: rcc::Clocks = rcc.cfgr.use_hse(25.mhz()).sysclk(SYSCLK_HZ.hz()).freeze();
        let dwt = ctx.core.DWT;
        let systick = ctx.core.SYST;
        let mono = DwtSystick::new(&mut dcb, dwt, systick, clocks.sysclk().0);

        // LED setup.
        let led: StatusLED = gpioc.pc13.into_push_pull_output();

        // Setup IR input.
        let mut ir_timer: IRTimer = timer::Timer::tim4(dp.TIM4, IR_SAMPLE_HZ.hz(), clocks);
        ir_timer.listen(timer::Event::TimeOut);
        let ir_input = gpiob.pb6.into_floating_input();

        // Setup IR receiver, indicating we will report deltas in microseconds.
        let ir_recv = infrared::hal::PeriodicReceiver::new(ir_input, IR_SAMPLE_HZ);

        defmt::info!("Hello from RTIC!");

        (
            init::LateResources {
                led,
                ir_recv,
                ir_timer,
            },
            init::Monotonics(mono),
        )
    }

    #[task(binds = TIM4, priority = 2, resources = [led, ir_recv, ir_timer])]
    fn poll_ir_input(ctx: poll_ir_input::Context) {
        let poll_ir_input::Resources {
            led,
            ir_recv,
            ir_timer,
        } = ctx.resources;
        (ir_timer, ir_recv, led).lock(
            |ir_timer: &mut IRTimer, ir_recv: &mut IRRecv, led: &mut StatusLED| {
                ir_timer.clear_interrupt(timer::Event::TimeOut);

                if let Ok(Some(cmd)) = ir_recv.poll() {
                    if let Some(button) = Apple2009::decode(cmd) {
                        defmt::info!("Button {} pressed", defmt::Debug2Format(&button));
                    }

                    // Blink LED to indicate IR received.
                    led.toggle().unwrap();
                }
            },
        );
    }

    #[idle]
    fn idle(_ctx: idle::Context) -> ! {
        loop {}
    }
}

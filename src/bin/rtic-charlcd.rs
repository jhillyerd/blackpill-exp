#![no_main]
#![no_std]

use blackpill_exp as _; // global logger + panicking-behavior + memory layout
use rtic;

#[rtic::app(device = stm32f4xx_hal::pac, dispatchers = [SPI1])]
mod app {
    use asm_delay::{bitrate, AsmDelay};
    use dwt_systick_monotonic::DwtSystick;
    use hd44780_driver::{CursorBlink, Display, DisplayMode, HD44780};
    use rtic::time::duration::*;
    use stm32f4xx_hal::{
        gpio::{gpioa, gpioc, Output, PushPull},
        prelude::*,
        rcc, stm32 as pac,
    };

    const SYSCLK_HZ: u32 = 100_000_000;

    #[monotonic(binds = SysTick, default = true)]
    type SysMono = DwtSystick<{ crate::app::SYSCLK_HZ }>;

    type StatusLED = gpioc::PC13<Output<PushPull>>;

    #[resources]
    struct Resources {
        #[init(0)]
        count: usize,
        delay: AsmDelay,
        led: crate::app::StatusLED,
        lcd: HD44780<
            hd44780_driver::bus::FourBitBus<
                gpioa::PA1<Output<PushPull>>,
                gpioa::PA2<Output<PushPull>>,
                gpioa::PA4<Output<PushPull>>,
                gpioa::PA5<Output<PushPull>>,
                gpioa::PA6<Output<PushPull>>,
                gpioa::PA7<Output<PushPull>>,
            >,
        >,
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

        // Display setup.
        let mut delay = AsmDelay::new(bitrate::Hertz(SYSCLK_HZ));
        let gpioa = dp.GPIOA.split();
        let rs = gpioa.pa1.into_push_pull_output();
        let en = gpioa.pa2.into_push_pull_output();
        let d4 = gpioa.pa4.into_push_pull_output();
        let d5 = gpioa.pa5.into_push_pull_output();
        let d6 = gpioa.pa6.into_push_pull_output();
        let d7 = gpioa.pa7.into_push_pull_output();
        let mut lcd = HD44780::new_4bit(rs, en, d4, d5, d6, d7, &mut delay).unwrap();
        lcd.reset(&mut delay).unwrap();
        lcd.clear(&mut delay).unwrap();
        lcd.set_display_mode(
            DisplayMode {
                display: Display::On,
                cursor_visibility: hd44780_driver::Cursor::Visible,
                cursor_blink: CursorBlink::On,
            },
            &mut delay,
        )
        .unwrap();
        lcd.write_str("Hello, world!", &mut delay).unwrap();

        blink_led::spawn().unwrap();

        defmt::info!("Hello from RTIC!");

        (
            init::LateResources { delay, led, lcd },
            init::Monotonics(mono),
        )
    }

    #[task(resources = [count, delay, led, lcd])]
    fn blink_led(ctx: blink_led::Context) {
        defmt::info!("Blink!");
        let blink_led::Resources {
            count,
            delay,
            led,
            lcd,
        } = ctx.resources;

        fn digit(d: usize) -> u8 {
            ('0' as u8) + d as u8
        }

        (count, delay, led, lcd).lock(|count, delay, led, lcd| {
            led.toggle().unwrap();
            *count += 1;
            lcd.set_cursor_pos(40, delay).ok();
            lcd.write_str("Count: ", delay).ok();
            lcd.write_byte(digit((*count / 1000) % 10), delay).ok();
            lcd.write_byte(digit((*count / 100) % 10), delay).ok();
            lcd.write_byte(digit((*count / 10) % 10), delay).ok();
            lcd.write_byte(digit((*count) % 10), delay).ok();
        });

        blink_led::spawn_after(Seconds(1_u32)).ok();
    }

    #[idle]
    fn idle(_ctx: idle::Context) -> ! {
        loop {}
    }
}

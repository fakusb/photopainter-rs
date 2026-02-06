#![no_std]
#![no_main]

mod epd;
mod rtc;
mod usb;
// mod usb_picotool_reset;

use core::time::Duration;
pub(crate) use embedded_hal_async::delay::DelayNs as _;

pub(crate) use defmt_or_log::*;
// use defmt_serial as _;
use embedded_hal_async::delay::*;

#[cfg(not(feature = "defmt"))]
use panic_halt as _;
#[cfg(feature = "defmt")]
use {defmt_rtt as _, panic_probe as _};

use embassy_rp::{
    Peri, dma,
    gpio::{Level, Output},
    peripherals::{self},
};
use embassy_time::Delay;
use static_cell::StaticCell;

embassy_rp::bind_interrupts!(
    struct Irqs {
        // PIO0_IRQ_0 => pio::InterruptHandler<PIO0>;
        // DMA_IRQ_0 => dma::InterruptHandler<peripherals::DMA_CH1>;
        // RTC_IRQ => rtc::InterruptHandler;
    }
);

static LOGLEVEL: log::LevelFilter = log::LevelFilter::Debug;
static EXECUTOR_LOW: StaticCell<embassy_executor::Executor> = StaticCell::new();

#[cortex_m_rt::entry]
fn main() -> ! {
    info!("main started");

    let p = embassy_rp::init(Default::default());

    let epd_power_enable = Output::new(p.PIN_16, Level::Low);

    // Low-priority executore
    let executor = EXECUTOR_LOW.init_with(embassy_executor::Executor::new);
    executor.run(|spawner| {
        unwrap!(spawner.spawn(blink(p.PIN_25)));
        unwrap!(spawner.spawn(reboot_on_bootsel(p.BOOTSEL)));
        usb::init(spawner, p.USB);
        unwrap!(spawner.spawn(epd::init(
            spawner,
            p.SPI1,
            p.DMA_CH1,
            p.PIN_12,
            p.PIN_8,
            p.PIN_9,
            p.PIN_13,
            p.PIN_10,
            p.PIN_11,
            epd_power_enable
        )));
    });
}

#[embassy_executor::task]
async fn blink(p: Peri<'static, peripherals::PIN_25>) {
    let mut p = Output::new(p, Level::Low);
    //let mut i: u16 = 0;
    loop {
        p.toggle();
        //i += 1;
        //info!("toggle {}", i);
        Delay.delay_ms(500).await;
    }
}

#[cfg(feature = "debug")]
#[embassy_executor::task]
async fn reboot_on_bootsel(mut p: Peri<'static, peripherals::BOOTSEL>) {
    loop {
        let p1 = p.reborrow();
        if embassy_rp::bootsel::is_bootsel_pressed(p1) {
            info!("Reboot into bootsel.");
            Delay.delay_ms(100).await;
            embassy_rp::rom_data::reset_to_usb_boot(0 << 15, 1);
        }
        Delay.delay_ms(200).await;
    }
}

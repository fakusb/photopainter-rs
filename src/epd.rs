#![allow(clippy::too_many_arguments)]

use defmt_or_log::{debug, info, warn};
use embassy_rp::{
    Peri,
    gpio::{Input, Level, Output, Pull},
    peripherals::{DMA_CH1, PIN_8, PIN_9, PIN_10, PIN_11, PIN_12, PIN_13, SPI1},
    spi::{Phase, Polarity, Spi},
};
use embassy_time::Delay;
use embedded_graphics::prelude::*;
use embedded_hal_async::delay::DelayNs;
use epd_waveshare::{epd7in3f::*, prelude::*};
use static_cell::StaticCell;
use unwrap_infallible::UnwrapInfallible;

#[embassy_executor::task]
pub async fn init(
    _spawner: embassy_executor::Spawner,
    spi0: Peri<'static, SPI1>,
    dma: Peri<'static, DMA_CH1>,
    rst: Peri<'static, PIN_12>,
    dc: Peri<'static, PIN_8>,
    cs: Peri<'static, PIN_9>,
    busy: Peri<'static, PIN_13>,
    clk: Peri<'static, PIN_10>,
    mosi: Peri<'static, PIN_11>,
    mut power_enable: Output<'static>,
) -> () {
    power_enable.set_high();
    Delay.delay_ms(1000).await; // to ensure logger is initialized
    warn!("TODO: override SINGLE_BYTE_WRITE");
    debug!("epd started!");
    let rst = Output::new(rst, Level::High);
    let dc = Output::new(dc, Level::High);
    let busy = Input::new(busy, Pull::Down);
    let cs = Output::new(cs, Level::High);

    debug!("epd pins!");

    let spi_bus = {
        let mut spi_conf: embassy_rp::spi::Config = Default::default();
        spi_conf.frequency = 2_000_000;
        spi_conf.phase = Phase::CaptureOnFirstTransition;
        spi_conf.polarity = Polarity::IdleLow;
        Spi::new_txonly(spi0, clk, mosi, dma, spi_conf)
    };

    let spi = &mut embedded_hal_bus::spi::ExclusiveDevice::new(spi_bus, cs, Delay).unwrap_infallible();

    let delay = &mut Delay;

    debug!("epd pre-await!");
    //static EPD: StaticCell<Epd7in3f<SPI1, _, _, _, _>> = StaticCell::new();
    /*EPD.init*/
    let mut epd = Epd7in3f::new(spi, busy, dc, rst, delay, None).await.unwrap();

    info!("epd init");

    //Use display graphics from embedded-graphics
    // static DISPLAY: StaticCell<Display7in3f> = StaticCell::new();
    // let display = DISPLAY.init_with(Default::default);
    epd.clear_frame(spi, delay).await.unwrap();

    info!("cleared");

    epd.sleep(spi, delay).await.unwrap();
    info!("sleep");

    power_enable.set_low();
    info!("epd disable power ");
}

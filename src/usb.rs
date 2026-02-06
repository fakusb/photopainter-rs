use defmt_or_log::*;
use embassy_rp::{peripherals::USB, usb};
use embassy_time::{Delay, Duration};
use embassy_usb::{
    UsbDevice,
    class::cdc_acm::{self, CdcAcmClass},
};
use static_cell::StaticCell;

// use crate::usb_picotool_reset::{self, State};

embassy_rp::bind_interrupts!(struct UsbIrqs {
    USBCTRL_IRQ => usb::InterruptHandler<USB>;
});

pub fn init(spawner: embassy_executor::Spawner, usb: embassy_rp::Peri<'static, USB>) {
    // Create the driver, from the HAL.
    let driver = usb::Driver::new(usb, UsbIrqs);

    // Create embassy-usb Config
    let config = {
        let mut config = embassy_usb::Config::new(0x2e8a, 0x000a);
        config.manufacturer = Some("Raspberry Pi");
        config.product = Some("Pico");
        config.serial_number = Some("12345678");
        config.max_power = 250;
        config.max_packet_size_0 = 64;
        config
    };

    // Create embassy-usb DeviceBuilder using the driver and config.
    // It needs some buffers for building the descriptors.
    let mut builder = {
        static CONFIG_DESCRIPTOR: StaticCell<[u8; 256]> = StaticCell::new();
        static BOS_DESCRIPTOR: StaticCell<[u8; 256]> = StaticCell::new();
        static CONTROL_BUF: StaticCell<[u8; 64]> = StaticCell::new();

        embassy_usb::Builder::new(
            driver,
            config,
            CONFIG_DESCRIPTOR.init([0; 256]),
            BOS_DESCRIPTOR.init([0; 256]),
            &mut [], // no msos descriptors
            CONTROL_BUF.init([0; 64]),
        )
    };

    // {
    //     static STATE: StaticCell<usb_picotool_reset::State<usb_picotool_reset::DefaultConfig>> = StaticCell::new();
    //     let state = STATE.init(usb_picotool_reset::State::new());
    //     usb_picotool_reset::configure(&mut builder, state);
    // };

    // Create classes on the builder.
    let class = {
        static STATE: StaticCell<cdc_acm::State> = StaticCell::new();
        let state = STATE.init(cdc_acm::State::new());
        CdcAcmClass::new(&mut builder, state, 64)
    };

    // Build the builder.
    let usb = builder.build();

    // // Run the USB device.
    unwrap!(spawner.spawn(usb_task(usb)));

    #[cfg(feature = "log")]
    {
        unwrap!(spawner.spawn(logger_init_task(class)));
    }
}

type MyUsbDriver = usb::Driver<'static, USB>;
type MyUsbDevice = UsbDevice<'static, MyUsbDriver>;

// #[embassy_executor::task]
// async fn log_connect_task(mut class: CdcAcmClass<'static, MyUsbDriver>) {
//     // Do stuff with the class!
//     loop {
//         class.wait_connection().await;
//         info!("USB Connected");
//         let _ = echo(&mut class).await;
//         info!("USB Disconnected");
//     }
// }

#[embassy_executor::task]
async fn usb_task(mut usb: MyUsbDevice) -> ! {
    loop {
        usb.run_until_suspend().await;
        usb.wait_resume().await;
    }
}

// struct Disconnected {}

// impl From<EndpointError> for Disconnected {
//     fn from(val: EndpointError) -> Self {
//         match val {
//             EndpointError::BufferOverflow => panic!("Buffer overflow"),
//             EndpointError::Disabled => Disconnected {},
//         }
//     }
// }

// async fn echo<'d, T: usb::Instance + 'd>(class: &mut CdcAcmClass<'d, usb::Driver<'d, T>>) -> Result<(), Disconnected> {
//     let mut buf = [0; 64];
//     loop {
//         let n = class.read_packet(&mut buf).await?;
//         let data = &buf[..n];
//         #[cfg(feature = "defmt")] // logging over serial otherwise loops
//         info!("USB data: {}", wrappers::Hex(data));
//         class.write_packet(data).await?;
//     }
// }
#[cfg(feature = "log")]
#[embassy_executor::task]
async fn logger_init_task(class: CdcAcmClass<'static, usb::Driver<'static, USB>>) {
    let fut = embassy_usb_logger::with_class!(1024, crate::LOGLEVEL, class);
    info!("Logger started;");
    fut.await;
}

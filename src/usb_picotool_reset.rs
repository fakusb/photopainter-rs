use defmt::warn;
use defmt_or_log::info;
use embassy_rp::rom_data::reset_to_usb_boot;
use embassy_usb::{
    Builder, Handler,
    control::{OutResponse, Recipient, Request, RequestType},
    driver::Driver,
    types::InterfaceNumber,
};

/// implements https://github.com/raspberrypi/pico-sdk/blob/master/src/rp2_common/pico_stdio_usb/reset_interface.c
/// copyed from https://github.com/ithinuel/usbd-picotool-reset
///
use core::{marker::PhantomData, mem::MaybeUninit};

use embassy_usb::types::StringIndex;

//use usb_device::LangID;
//use usb_device::class_prelude::{InterfaceNumber, StringIndex, UsbBus, UsbBusAllocator};

// Vendor specific class
const CLASS_VENDOR_SPECIFIC: u8 = 0xFF;
// cf: https://github.com/raspberrypi/pico-sdk/blob/f396d05f8252d4670d4ea05c8b7ac938ef0cd381/src/common/pico_usb_reset_interface/include/pico/usb_reset_interface.h#L17
const RESET_INTERFACE_SUBCLASS: u8 = 0x00;
const RESET_INTERFACE_PROTOCOL: u8 = 0x01;
const RESET_REQUEST_BOOTSEL: u8 = 0x01;
//const RESET_REQUEST_FLASH: u8 = 0x02;

/// Defines which feature of the bootloader are made available after reset.
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum DisableInterface {
    /// Both Mass Storage and Pico boot are enabled.
    None,
    /// Disables Mass Storage leaving only PicoBoot.
    DisableMassStorage,
    /// Disables PicoBoot leaving only Mass Storage.
    DisablePicoBoot,
}
impl DisableInterface {
    const fn into(self) -> u32 {
        match self {
            DisableInterface::None => 0,
            DisableInterface::DisableMassStorage => 1,
            DisableInterface::DisablePicoBoot => 2,
        }
    }
}

/// Allows to customize the configuration of the UsbClass.
pub trait Config {
    /// Configuration for which interface to enable/disable after reset.
    const INTERFACE_DISABLE: DisableInterface;
    /// Configuration for which pin to show mass storage activity after reset.
    const BOOTSEL_ACTIVITY_LED: Option<usize>;
}

/// Default configuration for PicoTool class.
///
/// This lets both interface enabled after reset and does not display mass storage activity on any
/// LED.
pub enum DefaultConfig {}
impl Config for DefaultConfig {
    const INTERFACE_DISABLE: DisableInterface = DisableInterface::None;

    const BOOTSEL_ACTIVITY_LED: Option<usize> = None;
}

pub struct State<C: Config> {
    control: MaybeUninit<Control<C>>,
}

struct Control<C: Config = DefaultConfig> {
    intf: InterfaceNumber,
    str_idx: StringIndex,
    _cnf: PhantomData<C>,
}

impl<C: Config> State<C> {
    /// Creates a new State
    pub fn new() -> State<C> {
        Self {
            control: MaybeUninit::uninit(),
        }
    }
}

impl<C: Config> Handler for Control<C> {
    fn get_string(&mut self, index: StringIndex, lang_id: u16) -> Option<&str> {
        (index == self.str_idx).then_some("Reset")
    }

    fn control_out(&mut self, req: Request, data: &[u8]) -> Option<OutResponse> {
        if !(req.request_type == RequestType::Class
            && req.recipient == Recipient::Interface
            && req.index == u8::from(self.intf) as u16)
        {
            return None;
        }

        match req.request {
            RESET_REQUEST_BOOTSEL => {
                let mut gpio_mask = C::BOOTSEL_ACTIVITY_LED.map(|led| 1 << led).unwrap_or(0);
                if req.value & 0x100 != 0 {
                    gpio_mask = 1 << (req.value >> 9);
                }
                embassy_rp::rom_data::reset_to_usb_boot(
                    gpio_mask,
                    u32::from(req.value & 0x7F) | C::INTERFACE_DISABLE.into(),
                );
                // no-need to accept/reject, we'll reset the device anyway
                unreachable!()
            }
            //RESET_REQUEST_FLASH => todo!(),
            req => {
                // we are not expecting any other USB OUT requests
                warn!("reset Request not implemented: {}", req);
                return Some(OutResponse::Rejected);
            }
        }
    }
}

pub fn configure<'d, C: Config, D: Driver<'d>>(builder: &mut Builder<'d, D>, state: &'d mut State<C>) {
    let iface_string = builder.string();

    let mut func = builder.function(
        CLASS_VENDOR_SPECIFIC,
        RESET_INTERFACE_SUBCLASS,
        RESET_INTERFACE_PROTOCOL,
    );

    let mut iface = func.interface();
    let comm_if = iface.interface_number();
    let mut _alt = iface.alt_setting(
        CLASS_VENDOR_SPECIFIC,
        RESET_INTERFACE_SUBCLASS,
        RESET_INTERFACE_PROTOCOL,
        Some(iface_string),
    );

    let control = state.control.write(Control {
        str_idx: iface_string,
        intf: comm_if,
        _cnf: PhantomData,
    });

    drop(func);

    builder.handler(control);
}

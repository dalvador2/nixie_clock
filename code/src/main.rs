#![no_std]
#![no_main]

use crate::tasks::{display::display, handler::handler, menu::menu, ntp::ntp};
use crate::utils::resources::{AssignedResources, DisplayResources, MenuResources, NTPResources};
use defmt::*;
use embassy_executor::Executor;
use embassy_executor::Spawner;
use embassy_rp::block::ImageDef;
use embassy_rp::gpio;
use embassy_rp::gpio::Input;
use embassy_rp::i2c::{Async, Instance};
use embassy_rp::multicore::{spawn_core1, Stack};
use embassy_rp::peripherals::I2C0;
use embassy_rp::{bind_interrupts, i2c};
use embassy_time::{Duration, Ticker, Timer};
use gpio::{Level, Output, Pull};
use pwm_pca9685::{Address, Channel, Pca9685};
use static_cell::StaticCell;
use tasks::display;
use {defmt_rtt as _, panic_probe as _};

mod tasks;
mod utils;
static mut CORE1_STACK: Stack<4096> = Stack::new();
static EXECUTOR0: StaticCell<Executor> = StaticCell::new();
static EXECUTOR1: StaticCell<Executor> = StaticCell::new();

#[link_section = ".start_block"]
#[used]
pub static IMAGE_DEF: ImageDef = ImageDef::secure_exe();

// Program metadata for `picotool info`.
// This isn't needed, but it's recomended to have these minimal entries.
#[link_section = ".bi_entries"]
#[used]
pub static PICOTOOL_ENTRIES: [embassy_rp::binary_info::EntryAddr; 4] = [
    embassy_rp::binary_info::rp_program_name!(c"high voltage enable"),
    embassy_rp::binary_info::rp_program_description!(
        c"This example tests enables hv on nixie clock"
    ),
    embassy_rp::binary_info::rp_cargo_version!(),
    embassy_rp::binary_info::rp_program_build_attribute!(),
];

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    let p = embassy_rp::init(Default::default());
    let r = split_resources!(p);
    spawner.spawn(display(r.display)).unwrap();
    spawner.spawn(ntp(r.ntp, spawner)).unwrap();
    spawner.spawn(menu(r.menu)).unwrap();
    spawner.spawn(handler()).unwrap();
}

// #[embassy_executor::task]
// async fn c1tasks(spawner: Spawner, disp_res: DisplayResources, menu_res: MenuResources) {
//     unwrap!(spawner.spawn(display(disp_res)));
//     unwrap!(spawner.spawn(menu(menu_res)));
//     unwrap!(spawner.spawn(handler()));
// }
//
// #[cortex_m_rt::entry]
// fn main() -> ! {
//     let p = embassy_rp::init(Default::default());
//     let r = split_resources!(p);
//
//     spawn_core1(
//         p.CORE1,
//         unsafe { &mut *core::ptr::addr_of_mut!(CORE1_STACK) },
//         move || {
//             let executor1 = EXECUTOR1.init(Executor::new());
//             executor1.run(|spawner| unwrap!(spawner.spawn(c1tasks(spawner, r.display, r.menu))));
//         },
//     );
//
//     let executor0 = EXECUTOR0.init(Executor::new());
//     executor0.run(|spawner| unwrap!(spawner.spawn(ntp(r.ntp, spawner))));
// }

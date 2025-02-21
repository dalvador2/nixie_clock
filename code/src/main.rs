//! This example test the RP Pico on board LED.
//!
//! It does not work with the RP Pico W board. See wifi_blinky.rs.

#![no_std]
#![no_main]

use defmt::*;
use embassy_executor::Spawner;
use embassy_rp::block::ImageDef;
use embassy_rp::gpio;
use embassy_rp::gpio::Input;
use embassy_rp::i2c::{Async, Instance};
use embassy_rp::peripherals::I2C0;
use embassy_rp::{bind_interrupts, i2c};
use embassy_time::{Duration, Ticker, Timer};
use gpio::{Level, Output, Pull};
use pwm_pca9685::{Address, Channel, Pca9685};
use {defmt_rtt as _, panic_probe as _};

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

bind_interrupts!(struct Irqs {
    I2C0_IRQ => i2c::InterruptHandler<I2C0>;
}
);

#[derive(Format, Copy, Clone)]
pub struct NixieState {
    digits: [u8; 6],
    commas: [bool; 12],
}
impl NixieState {
    pub fn new(digits: [u8; 6], commas: [bool; 12]) -> Self {
        Self { digits, commas }
    }

    // pub fn from_usize(number: u32) -> Self {
    //     let digits = [0u8; 6];
    //     for i in 0i32..6 {}
    //     Self {
    //         digits: digits,
    //         commas: [false; 12],
    //     }
    // }
    pub fn blank() -> Self {
        Self {
            digits: [10u8; 6],
            commas: [false; 12],
        }
    }
}
impl Default for NixieState {
    fn default() -> Self {
        NixieState {
            digits: [0u8; 6],
            commas: [false; 12],
        }
    }
}

pub struct Display<'a, T>
where
    T: Instance,
{
    current_state: NixieState,
    previous_state: NixieState,
    i2c_dev: i2c::I2c<'a, T, Async>,
    digitmap: [[(Address, Channel); 10]; 6],
    commamap: [(Address, Channel); 12],
}

impl<'a, T> Display<'a, T>
where
    T: Instance,
{
    fn new(
        i2c_dev: i2c::I2c<'a, T, Async>,
        digitmap: [[(Address, Channel); 10]; 6],
        commamap: [(Address, Channel); 12],
    ) -> Self {
        Display {
            current_state: NixieState::default(),
            previous_state: NixieState::blank(),
            i2c_dev,
            digitmap,
            commamap,
        }
    }
    async fn setup(mut self) -> Self {
        for i in 65u8..=69 {
            let mut pwm = Pca9685::new(self.i2c_dev, Address::from(i)).unwrap();
            pwm.enable().await.unwrap();
            pwm.set_prescale(100).await.unwrap();
            self.i2c_dev = pwm.destroy();
        }
        self
    }
    async fn wipe(mut self) -> Self {
        for i in 65u8..=69 {
            let mut pwm = Pca9685::new(self.i2c_dev, Address::from(i)).unwrap();
            pwm.set_channel_on_off(Channel::All, 0, 0).await.unwrap();
            pwm.enable().await.unwrap();
            self.i2c_dev = pwm.destroy()
        }
        self
    }
    async fn show(mut self, state: NixieState, init: bool, brightness: u16) -> Self {
        self.previous_state = self.current_state;
        self.current_state = state;
        for (digit, digit_int) in self.previous_state.digits.iter().enumerate() {
            let next_digit_int = &self.current_state.digits[digit];
            if !init {
                if *digit_int == 10u8 {
                    continue;
                }
                if *next_digit_int == 10u8 {
                    continue;
                }
                if *digit_int == *next_digit_int {
                    continue;
                }
            }
            let (address, channel): (Address, Channel) = self.digitmap[digit][*digit_int as usize];
            let mut pwm = Pca9685::new(self.i2c_dev, address).unwrap();
            pwm.enable().await.unwrap();
            pwm.set_channel_on_off(channel, 0, 0).await.unwrap();
            let (address, channel): (Address, Channel) =
                self.digitmap[digit][*next_digit_int as usize];
            self.i2c_dev = pwm.destroy();
            let mut pwm = Pca9685::new(self.i2c_dev, address).unwrap();
            pwm.enable().await.unwrap();
            pwm.set_channel_on_off(channel, 0, brightness)
                .await
                .unwrap();
            self.i2c_dev = pwm.destroy();
        }
        for (comma_no, on_off) in self.current_state.commas.iter().enumerate() {
            let p_on_off = self.previous_state.commas[comma_no];
            if on_off ^ p_on_off {
                let (address, channel): (Address, Channel) = self.commamap[comma_no];
                let mut pwm = Pca9685::new(self.i2c_dev, address).unwrap();
                pwm.enable().await.unwrap();
                if *on_off {
                    pwm.set_channel_on_off(channel, 0, brightness)
                        .await
                        .unwrap();
                } else {
                    pwm.set_channel_on_off(channel, 0, 0).await.unwrap();
                }
                self.i2c_dev = pwm.destroy();
            }
        }
        self
    }
}

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    let a = [
        Address::from(65u8),
        Address::from(66u8),
        Address::from(67u8),
        Address::from(68u8),
        Address::from(69u8),
    ];
    let digit_map = [
        [
            (a[4], Channel::C1),
            (a[4], Channel::C0),
            (a[4], Channel::C9),
            (a[4], Channel::C8),
            (a[4], Channel::C7),
            (a[4], Channel::C6),
            (a[4], Channel::C5),
            (a[4], Channel::C4),
            (a[4], Channel::C3),
            (a[4], Channel::C2),
        ],
        [
            (a[3], Channel::C1),
            (a[3], Channel::C0),
            (a[3], Channel::C9),
            (a[3], Channel::C8),
            (a[3], Channel::C7),
            (a[3], Channel::C6),
            (a[3], Channel::C5),
            (a[3], Channel::C4),
            (a[3], Channel::C3),
            (a[3], Channel::C2),
        ],
        [
            (a[2], Channel::C1),
            (a[2], Channel::C0),
            (a[2], Channel::C9),
            (a[2], Channel::C8),
            (a[2], Channel::C7),
            (a[2], Channel::C6),
            (a[2], Channel::C5),
            (a[2], Channel::C4),
            (a[2], Channel::C3),
            (a[2], Channel::C2),
        ],
        [
            (a[1], Channel::C1),
            (a[1], Channel::C0),
            (a[1], Channel::C9),
            (a[1], Channel::C8),
            (a[1], Channel::C7),
            (a[1], Channel::C6),
            (a[1], Channel::C5),
            (a[1], Channel::C4),
            (a[1], Channel::C3),
            (a[1], Channel::C2),
        ],
        [
            (a[0], Channel::C1),
            (a[0], Channel::C0),
            (a[0], Channel::C9),
            (a[0], Channel::C8),
            (a[0], Channel::C7),
            (a[0], Channel::C6),
            (a[0], Channel::C5),
            (a[0], Channel::C4),
            (a[0], Channel::C3),
            (a[0], Channel::C2),
        ],
        [
            (a[2], Channel::C13),
            (a[2], Channel::C12),
            (a[1], Channel::C13),
            (a[1], Channel::C12),
            (a[0], Channel::C15),
            (a[0], Channel::C14),
            (a[0], Channel::C13),
            (a[0], Channel::C12),
            (a[2], Channel::C15),
            (a[2], Channel::C14),
        ],
    ];
    let comma_map = [
        (a[4], Channel::C11),
        (a[4], Channel::C10),
        (a[3], Channel::C11),
        (a[3], Channel::C10),
        (a[2], Channel::C11),
        (a[2], Channel::C10),
        (a[1], Channel::C11),
        (a[1], Channel::C10),
        (a[0], Channel::C11),
        (a[0], Channel::C10),
        (a[1], Channel::C15),
        (a[1], Channel::C14),
    ];
    let p = embassy_rp::init(Default::default());
    let mut gp3 = Output::new(p.PIN_3, Level::Low);
    let mut button = Input::new(p.PIN_7, Pull::Up);
    let mut ext_clk = Output::new(p.PIN_2, Level::Low);
    ext_clk.set_low();
    gp3.set_high();
    let mut i2c_config = i2c::Config::default();
    i2c_config.frequency = 1_000_000;
    let mut dev = i2c::I2c::new_async(p.I2C0, p.PIN_21, p.PIN_20, Irqs, i2c_config);
    let mut disp = Display::new(dev, digit_map, comma_map);
    disp = disp.setup().await;
    disp = disp.wipe().await;
    let mut ticker = Ticker::every(Duration::from_hz(12));
    let brightness = 4095;
    Timer::after_secs(2).await;
    disp = disp
        .show(
            NixieState::new([0, 0, 0, 0, 0, 0], [false; 12]),
            true,
            brightness,
        )
        .await;
    for hour in 0usize..=59 {
        for minute in 0usize..=59 {
            for second in 0usize..=59 {
                let mut disp_int = ((hour * 100) + minute) * 100 + second;
                let mut disp_nixie_state = NixieState::new([0; 6], [false; 12]);
                for i in 0..6 {
                    let k = disp_int / 10;
                    disp_nixie_state.digits[5 - i] = (disp_int - (k * 10)) as u8;
                    disp_int = k;
                }
                for disp_comma in 0usize..=11 {
                    disp_nixie_state.commas[disp_comma] = true;
                    disp = disp.show(disp_nixie_state, false, brightness).await;
                    ticker.next().await;
                }
            }
        }
    }
    loop {
        Timer::after_millis(500).await;
    }
}

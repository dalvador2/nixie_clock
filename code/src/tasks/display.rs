#![no_std]
#![no_main]

use core::usize;

use crate::utils::mutex_channels::DISPLAY_MUT;
use crate::utils::resources::{AssignedResources, DisplayResources};
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

bind_interrupts!(struct Irqs {
    I2C0_IRQ => i2c::InterruptHandler<I2C0>;
}
);
#[derive(Format)]
pub struct NixieDispCommand {
    pub brightness: usize,
    pub nixie_state: NixieState,
}

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
    //     let digits =k [0u8; 6];
    //     for i in 0i32..6 {}
    //     Self {
    //         digits: digits,
    //         commas: [false; 12],
    //     }
    // }
    pub fn from_hmsc(hours: u32, mins: u32, seconds: u32, commas: [bool; 12]) -> Self {
        let digits = [
            (hours / 10) as u8,
            (hours % 10) as u8,
            (mins / 10) as u8,
            (mins % 10) as u8,
            (seconds / 10) as u8,
            (seconds % 10) as u8,
        ];
        Self { digits, commas }
    }
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
    pub fn new(
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
    pub async fn setup(mut self) -> Self {
        for i in 65u8..=69 {
            let mut pwm = Pca9685::new(self.i2c_dev, Address::from(i)).unwrap();
            pwm.enable().await.unwrap();
            pwm.set_prescale(100).await.unwrap();
            self.i2c_dev = pwm.destroy();
        }
        self
    }
    pub async fn wipe(mut self) -> Self {
        for i in 65u8..=69 {
            let mut pwm = Pca9685::new(self.i2c_dev, Address::from(i)).unwrap();
            pwm.set_channel_on_off(Channel::All, 0, 0).await.unwrap();
            pwm.enable().await.unwrap();
            self.i2c_dev = pwm.destroy()
        }
        self
    }
    pub async fn show(mut self, state: NixieState, init: bool, brightness: u16) -> Self {
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

#[embassy_executor::task]
pub async fn display(r: DisplayResources) {
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
    let mut i2c_config = i2c::Config::default();
    i2c_config.frequency = 1_000_000;
    let mut dev = i2c::I2c::new_async(r.peri, r.scl, r.sdi, Irqs, i2c_config);
    let mut ext_clk = Output::new(r.nixieclk, Level::Low);
    let mut hv_en = Output::new(r.hv_en, Level::Low);
    let mut disp = Display::new(dev, digit_map, comma_map);
    hv_en.set_high();
    ext_clk.set_low();
    disp = disp.setup().await;
    disp = disp.wipe().await;
    let mut first = true;
    loop {
        let result = DISPLAY_MUT.receive().await;
        debug!("{:?}", result);
        disp = disp
            .show(
                result.nixie_state,
                first,
                result.brightness.try_into().unwrap(),
            )
            .await;
        first = false;
    }
}

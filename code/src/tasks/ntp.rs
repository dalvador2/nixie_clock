use core::net::SocketAddr;

use crate::tasks::handler::{HandlerTime, NixieHandlerCommand};
use crate::utils::{
    mutex_channels::{HANDLER_MUT, NTP_MUT},
    resources::NTPResources,
};
use core::env;
use cyw43::JoinOptions;
use cyw43_pio::{PioSpi, RM2_CLOCK_DIVIDER};
use defmt::*;
use embassy_executor;
use embassy_executor::Spawner;
use embassy_net::udp::{PacketMetadata, UdpSocket};
use embassy_net::{dns::DnsQueryType, Config, StackResources};
use embassy_rp::bind_interrupts;
use embassy_rp::clocks::RoscRng;
use embassy_rp::gpio::{Level, Output};
use embassy_rp::peripherals::{DMA_CH0, PIO0};
use embassy_rp::pio::{InterruptHandler, Pio};
use embassy_time::{Duration, Ticker, Timer};
use rand::RngCore;
use sntpc::{get_time, NtpContext, NtpTimestampGenerator};
use static_cell::StaticCell;
use {defmt_rtt as _, panic_probe as _};

pub struct NixieNPTCommand {
    pub ticker_duration: Duration,
}

#[derive(Copy, Clone)] //todo change to seconds and micros
pub struct Timestamp {
    pub seconds: u64,
    pub micros: u32,
}
impl NtpTimestampGenerator for Timestamp {
    fn init(&mut self) {}

    fn timestamp_sec(&self) -> u64 {
        self.seconds
    }

    fn timestamp_subsec_micros(&self) -> u32 {
        self.micros
    }
}

impl Default for Timestamp {
    fn default() -> Self {
        Self {
            seconds: 0u64,
            micros: 0u32,
        }
    }
}
impl Timestamp {
    pub fn new(seconds: u64, micros: u32) -> Self {
        Self { seconds, micros }
    }
}

bind_interrupts!(struct Irqs {
    PIO0_IRQ_0 => InterruptHandler<PIO0>;
});

const WIFI_NETWORK: &str = env!("NIXIE_SSID");
const WIFI_PASSWORD: &str = env!("NIXIE_PASS");

#[embassy_executor::task]
async fn cyw43_task(
    runner: cyw43::Runner<'static, Output<'static>, PioSpi<'static, PIO0, 0, DMA_CH0>>,
) -> ! {
    runner.run().await
}

#[embassy_executor::task]
async fn net_task(mut runner: embassy_net::Runner<'static, cyw43::NetDriver<'static>>) -> ! {
    runner.run().await
}

#[embassy_executor::task]
pub async fn ntp(r: NTPResources, spawner: Spawner) {
    info!("Hello World!");

    let mut rng = RoscRng;

    let fw = include_bytes!("../../firmware/43439A0.bin");
    let clm = include_bytes!("../../firmware/43439A0_clm.bin");

    // To make flashing faster for development, you may want to flash the firmwares independently
    // at hardcoded addresses, instead of baking them into the program with `include_bytes!`:
    //     probe-rs download 43439A0.bin --binary-format bin --chip RP2040 --base-address 0x10100000
    //     probe-rs download 43439A0_clm.bin --binary-format bin --chip RP2040 --base-address 0x10140000
    //let fw = unsafe { core::slice::from_raw_parts(0x10100000 as *const u8, 230321) };
    //let clm = unsafe { core::slice::from_raw_parts(0x10140000 as *const u8, 4752) };

    let pwr = Output::new(r.pwr, Level::Low);
    let cs = Output::new(r.cs, Level::High);
    let mut pio = Pio::new(r.pio, Irqs);
    let spi = PioSpi::new(
        &mut pio.common,
        pio.sm0,
        RM2_CLOCK_DIVIDER,
        pio.irq0,
        cs,
        r.clk,
        r.dio,
        r.dma,
    );

    static STATE: StaticCell<cyw43::State> = StaticCell::new();
    let state = STATE.init(cyw43::State::new());
    let (net_device, mut control, runner) = cyw43::new(state, pwr, spi, fw).await;
    unwrap!(spawner.spawn(cyw43_task(runner)));

    control.init(clm).await;
    control
        .set_power_management(cyw43::PowerManagementMode::PowerSave)
        .await;

    let config = Config::dhcpv4(Default::default());
    //let config = embassy_net::Config::ipv4_static(embassy_net::StaticConfigV4 {
    //    address: Ipv4Cidr::new(Ipv4Address::new(192, 168, 69, 2), 24),
    //    dns_servers: Vec::new(),
    //    gateway: Some(Ipv4Address::new(192, 168, 69, 1)),
    //});

    // Generate random seed
    let seed = rng.next_u64();

    // Init network stack
    static RESOURCES: StaticCell<StackResources<3>> = StaticCell::new();
    let (stack, runner) = embassy_net::new(
        net_device,
        config,
        RESOURCES.init(StackResources::new()),
        seed,
    );

    unwrap!(spawner.spawn(net_task(runner)));

    loop {
        match control
            .join(WIFI_NETWORK, JoinOptions::new(WIFI_PASSWORD.as_bytes()))
            .await
        {
            Ok(_) => break,
            Err(err) => {
                info!("join failed with status={}", err.status);
            }
        }
    }

    // Wait for DHCP, not necessary when using static IP
    info!("waiting for DHCP...");
    while !stack.is_config_up() {
        Timer::after_millis(100).await;
    }
    info!("DHCP is now up!");
    let hwadd = stack.hardware_address();
    let ipadd = stack.config_v4();
    let ip6add = stack.config_v6();
    info!(
        "Mac: {} | IP: {} | IP6: {}",
        Debug2Format(&hwadd),
        Debug2Format(&ipadd),
        Debug2Format(&ip6add)
    );

    // And now we can use it!

    let mut rx_buffer = [0; 4096];
    let mut tx_buffer = [0; 4096];
    let mut rx_meta = [PacketMetadata::EMPTY; 16];
    let mut tx_meta = [PacketMetadata::EMPTY; 16];

    let mut socket = UdpSocket::new(
        stack,
        &mut rx_meta,
        &mut rx_buffer,
        &mut tx_meta,
        &mut tx_buffer,
    );
    let server_address = stack
        .dns_query("pool.ntp.org", DnsQueryType::A)
        .await
        .unwrap()[0];
    let socket_addr = SocketAddr::new(server_address.into(), 123);
    let socket_result = socket.bind(0).unwrap();
    info!("socket result{:?}", socket_result);
    let mut tick_duration: Duration = NTP_MUT.receive().await.ticker_duration;
    let mut ticker = Ticker::every(tick_duration);
    let mut time = 0u64;
    let mut time_micros = 0u32;
    let mut ptime = time;
    let mut first = true;
    loop {
        if (time > ptime + 1024) | first {
            let context = NtpContext::new(Timestamp::new(time, time_micros));
            let response = get_time(socket_addr, &socket, context).await.unwrap();
            info!("response{:?}", response);
            time = response.sec().try_into().unwrap();
            time_micros = response.sec_fraction() / ((u64::pow(2, 32) / 1000000u64) as u32);
            ptime = time;
            first = false
        }
        HANDLER_MUT
            .send(NixieHandlerCommand::DispTime(HandlerTime {
                seconds: time,
                micros: time_micros,
            }))
            .await;
        time += tick_duration.as_secs();
        time_micros = time_micros + tick_duration.as_micros() as u32 % 1000000;
        time += time_micros as u64 / 1000000;
        time_micros = time_micros % 1000000;
        ticker = match NTP_MUT.try_receive() {
            Ok(command) => {
                tick_duration = command.ticker_duration;
                Ticker::every(command.ticker_duration)
            }
            Err(_) => ticker,
        };
        ticker.next().await;
    }
}

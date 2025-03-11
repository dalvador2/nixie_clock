use crate::tasks::ntp::NixieNPTCommand;
use crate::utils::mutex_channels::*;
use chrono::{DateTime, Timelike};
use core::cmp::min;
use defmt::debug;
use defmt::*;
use embassy_executor;
use embassy_time::Duration;
use sntpc::NtpResult;

use super::display::{NixieDispCommand, NixieState};

pub enum NixieHandlerCommand {
    DispTime(HandlerTime),
}
#[derive(Debug, Format)]
pub struct HandlerTime {
    pub seconds: u64,
    pub micros: u32,
}

#[embassy_executor::task]
pub async fn handler() {
    NTP_MUT
        .send(NixieNPTCommand {
            ticker_duration: Duration::from_hz(12),
        })
        .await;
    loop {
        let message = HANDLER_MUT.receive().await;
        match message {
            NixieHandlerCommand::DispTime(handler_time) => {
                debug!("{:?}", handler_time);
                let dt = DateTime::from_timestamp(
                    handler_time.seconds.try_into().unwrap(),
                    handler_time.micros * 1000,
                )
                .unwrap();
                let hour = dt.hour();
                let minute = dt.minute();
                let seconds = dt.second();
                let twelths = min((12 * dt.timestamp_subsec_millis()) / 1000, 11) as usize;
                let mut commas = [false; 12];
                commas[twelths] = true;
                let nixie_state = NixieState::from_hmsc(hour, minute, seconds, commas);
                let send_state = NixieDispCommand {
                    brightness: 4095,
                    nixie_state,
                };
                DISPLAY_MUT.send(send_state).await;
            }
        }
    }
}

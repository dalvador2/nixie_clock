use crate::tasks::display::NixieDispCommand;
use crate::tasks::handler::NixieHandlerCommand;
use crate::tasks::menu::NixieMenuCommand;
use crate::tasks::ntp::NixieNPTCommand;
use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, channel::Channel};

pub static DISPLAY_MUT: Channel<CriticalSectionRawMutex, NixieDispCommand, 5> = Channel::new();
pub static MENU_MUT: Channel<CriticalSectionRawMutex, NixieMenuCommand, 5> = Channel::new();
pub static NTP_MUT: Channel<CriticalSectionRawMutex, NixieNPTCommand, 5> = Channel::new();
pub static HANDLER_MUT: Channel<CriticalSectionRawMutex, NixieHandlerCommand, 5> = Channel::new();

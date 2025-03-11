use crate::utils::resources::MenuResources;
use embassy_executor;

pub struct NixieMenuCommand {
    test_field: u32,
}

#[embassy_executor::task]
pub async fn menu(r: MenuResources) {}

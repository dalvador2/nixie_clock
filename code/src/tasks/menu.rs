use crate::utils::resources::MenuResources;
use core::ops::{Deref, DerefMut};
use embassy_executor;
use embassy_rp::gpio::{Input, Level, Output, Pull};
use no_alloc::BoxS;

pub struct NixieMenu {
    pub submenu_items: [BoxS<Option<NixieMenu>, [usize; 2048]>; 10],
    pub active_item: Option<usize>,
    pub active: bool,
    pub id: usize,
    pub endpoint: bool,
    pub value: Option<usize>,
    pub display_func: BoxS<dyn Fn(&Option<usize>, &Option<usize>) + Send, [usize; 2048]>,
}
impl NixieMenu {
    fn next(&mut self) {
        if let Some(active_item) = self.active_item {
            if self.active {
                if self.endpoint {
                    if let Some(value) = self.value {
                        self.value = Some(value + 1);
                    }
                } else {
                    self.active_item = Some((active_item + 1) % 10);
                };
            } else {
                if let Some(item) = self.submenu_items[active_item].deref_mut() {
                    item.next();
                }
            }
        }
    }
    fn previous(&mut self) {
        if let Some(active_item) = self.active_item {
            if self.active {
                if self.endpoint {
                    if let Some(value) = self.value {
                        self.value = Some(value - 1);
                    }
                } else {
                    self.active_item = Some((active_item + 10 - 1) % 10);
                };
            } else {
                if let Some(item) = self.submenu_items[active_item].deref_mut() {
                    item.previous();
                }
            }
        }
    }
    fn step_in(&mut self) -> Result<(), ()> {
        if let Some(active_item) = self.active_item {
            match self.submenu_items[active_item].deref_mut() {
                None => Err(()),
                Some(sub_item) => {
                    if self.active {
                        self.active = false;
                        sub_item.active = true;
                    } else {
                        sub_item.step_in();
                    }
                    Ok(())
                }
            }
        } else {
            Err(())
        }
    }
    fn step_out(&mut self) -> Result<(), ()> {
        if let Some(active_item) = self.active_item {
            match self.submenu_items[active_item].deref_mut() {
                None => Err(()),
                Some(sub_item) => {
                    if sub_item.active {
                        self.active = true;
                        sub_item.active = false;
                    } else {
                        sub_item.step_out();
                    }
                    Ok(())
                }
            }
        } else {
            Err(())
        }
    }
    fn display(&self) {
        self.display_func.deref()(&self.active_item, &self.value);
    }
    fn rec_display(&self) {
        if self.active {
            self.display_func.deref()(&self.active_item, &self.value);
        } else {
            if let Some(active_item) = self.active_item {
                if let Some(sub_item) = self.submenu_items[active_item].deref() {
                    sub_item.rec_display();
                }
            }
        }
    }
}

#[embassy_executor::task]
pub async fn menu(r: MenuResources) {
    let disp = |active_item, value| ();
    let mut menu = NixieMenu {
        active: false,
        active_item: None,
        submenu_items: [
            BoxS::new(None),
            BoxS::new(None),
            BoxS::new(None),
            BoxS::new(None),
            BoxS::new(None),
            BoxS::new(None),
            BoxS::new(None),
            BoxS::new(None),
            BoxS::new(None),
            BoxS::new(None),
        ],
        id: 0,
        endpoint: true,
        value: Some(0),
        display_func: BoxS::new(disp),
    };
    let mut hv_en = Output::new(r.hv_en, Level::Low);
    let mut b1 = Input::new(r.b1, Pull::Up);
    let mut b2 = Input::new(r.b2, Pull::Up);
    let mut b3 = Input::new(r.b3, Pull::Up);
    b2.wait_for_high().await;
    hv_en.set_high();
}

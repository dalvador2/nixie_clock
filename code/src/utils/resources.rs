use assign_resources::assign_resources;
use embassy_rp::peripherals;

assign_resources! {
    display: DisplayResources{
        peri: I2C0,
        scl: PIN_21,
        sdi: PIN_20,
        nixieclk: PIN_2,
    },
    menu: MenuResources{
        b1: PIN_6,
        b2: PIN_7,
        b3: PIN_8,
        hv_en: PIN_3,
    }
    ntp: NTPResources{
        pwr: PIN_23,
        cs: PIN_25,
        pio: PIO0,
        dio: PIN_29,
        clk: PIN_24,
        dma: DMA_CH0,
    }
}

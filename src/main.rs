#![no_main]
#![no_std]

use bsp::hal::{
    gpio::FunctionUart,
    uart::{self, UartPeripheral},
    Clock,
};
use defmt_rtt as _;
use embedded_hal::{digital::v2::OutputPin, spi::MODE_0};
use embedded_sdmmc::{Controller, TimeSource, VolumeIdx};
use fugit::RateExtU32;
use hal::{
    gpio::{FunctionSpi, Pins},
    Sio, Spi,
};
use panic_probe as _;
use rp_pico::{self as bsp, hal, hal::pac};

mod fname;

#[defmt::panic_handler]
fn panic() -> ! {
    cortex_m::asm::udf()
}

struct FrozenClock;
impl TimeSource for FrozenClock {
    fn get_timestamp(&self) -> embedded_sdmmc::Timestamp {
        embedded_sdmmc::Timestamp {
            year_since_1970: 0,
            zero_indexed_month: 0,
            zero_indexed_day: 0,
            hours: 0,
            minutes: 0,
            seconds: 0,
        }
    }
}

#[cortex_m_rt::entry]
fn main() -> ! {
    let core = pac::CorePeripherals::take().unwrap();
    let mut _delay = cortex_m::delay::Delay::new(core.SYST, bsp::XOSC_CRYSTAL_FREQ);
    let mut p = pac::Peripherals::take().unwrap();
    let mut watchdog = hal::Watchdog::new(p.WATCHDOG);
    let clocks = hal::clocks::init_clocks_and_plls(
        bsp::XOSC_CRYSTAL_FREQ,
        p.XOSC,
        p.CLOCKS,
        p.PLL_SYS,
        p.PLL_USB,
        &mut p.RESETS,
        &mut watchdog,
    )
    .ok()
    .unwrap();
    let sio = Sio::new(p.SIO);
    let pins = Pins::new(p.IO_BANK0, p.PADS_BANK0, sio.gpio_bank0, &mut p.RESETS);
    let mut cs = pins.gpio26.into_push_pull_output();
    cs.set_high().unwrap();
    let _spi_sclk = pins.gpio2.into_mode::<FunctionSpi>();
    let _spi_mosi = pins.gpio3.into_mode::<FunctionSpi>();
    let _spi_miso = pins.gpio4.into_mode::<FunctionSpi>();
    let spi = Spi::<_, _, 8>::new(p.SPI0).init(
        &mut p.RESETS,
        clocks.peripheral_clock.freq(),
        20.MHz(),
        &MODE_0,
    );
    let mut sdmmc = embedded_sdmmc::SdMmcSpi::new(spi, cs);
    let block_spi = sdmmc.acquire().unwrap();
    let mut ctrl = Controller::<_, _, 4, 4>::new(block_spi, FrozenClock);
    let mut vol = ctrl.get_volume(VolumeIdx(0)).unwrap();
    let root = ctrl.open_root_dir(&vol).unwrap();
    let mut builder = fname::Builder::new();
    ctrl.iterate_dir(&vol, &root, |entry| {
        builder.push(entry.name.base_name(), entry.name.extension());
    })
    .unwrap();
    let file_name = builder.finish().into_inner();
    let mut file = ctrl
        .open_file_in_dir(
            &mut vol,
            &root,
            core::str::from_utf8(&file_name).unwrap(),
            embedded_sdmmc::Mode::ReadWriteCreateOrAppend,
        )
        .unwrap();

    let pins = (
        pins.gpio0.into_mode::<FunctionUart>(),
        pins.gpio1.into_mode::<FunctionUart>(),
    );
    let uart = UartPeripheral::new(p.UART0, pins, &mut p.RESETS)
        .enable(
            uart::common_configs::_9600_8_N_1,
            clocks.peripheral_clock.freq(),
        )
        .unwrap();
    let mut buf = [0u8; 1024];
    let mut ptr = 0usize;
    loop {
        if let Some(read) = nb::block!(uart.read_raw(&mut buf[ptr..])).ok() {
            ptr += read;
            if ptr == buf.len() {
                ctrl.write(&mut vol, &mut file, &buf).unwrap();
                ptr = 0;
                defmt::println!("written");
            }
        } else {
            defmt::println!("error");
        }
    }
}

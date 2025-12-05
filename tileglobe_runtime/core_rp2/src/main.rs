#![no_std]
#![no_main]

extern crate alloc;

use core::net::SocketAddr;
use defmt::*;
use embassy_executor::Executor;
use embassy_executor::Spawner;
use embassy_rp::gpio::Output;
use embassy_rp::gpio::{Input, Level, Pull};
use embassy_rp::peripherals::{DMA_CH0, DMA_CH15, PIO2};
use embassy_rp::pio::InterruptHandler;
use embassy_rp::pio::Pio;
use embassy_rp::{Peripherals, bind_interrupts};
use embassy_time::{Duration, Ticker, Timer};
use static_cell::StaticCell;

use {defmt_rtt as _, panic_probe as _};

#[unsafe(link_section = ".bi_entries")]
#[used]
pub static PICOTOOL_ENTRIES: [embassy_rp::binary_info::EntryAddr; 4] = [
    embassy_rp::binary_info::rp_program_name!(c"Blinky Example"),
    embassy_rp::binary_info::rp_program_description!(
        c"This example tests the RP Pico on board LED, connected to gpio 25"
    ),
    embassy_rp::binary_info::rp_cargo_version!(),
    embassy_rp::binary_info::rp_program_build_attribute!(),
];

use embassy_net::Stack;
use embassy_net::tcp::{TcpReader, TcpSocket, TcpWriter};
use embassy_rp::adc::Adc;
use embassy_rp::clocks::RoscRng;
use embassy_sync::blocking_mutex::raw::{CriticalSectionRawMutex, NoopRawMutex};
use embedded_alloc::LlffHeap as Heap;
use fixed::FixedU32;
use embassy_rp::spinlock_mutex::SpinlockRawMutex;
use tileglobe::world::block::BlockState;
use tileglobe::world::chunk::Chunk;
use tileglobe::world::world::{LocalWorld, World};
use tileglobe::world::{BlockPos, ChunkLocalPos, ChunkPos};
use tileglobe_server::MCClient;
use tileglobe_server::mc_server::MCServer;

#[global_allocator]
static HEAP: Heap = Heap::empty();

#[embassy_executor::task(pool_size = 1)]
async fn main_task(spawner: Spawner, ps: Peripherals) -> ! {
    {
        embassy_rp::psram::Psram::new(
            embassy_rp::qmi_cs1::QmiCs1::new(ps.QMI_CS1, ps.PIN_0),
            embassy_rp::psram::Config::aps6404l(),
        )
            .expect("Failed to initialize PSRAM");

        unsafe extern "C" {
            static __psram_heap_start: u8;
            static __psram_heap_end: u8;
        }

        let start = unsafe { &__psram_heap_start as *const u8 as usize };
        let end = unsafe { &__psram_heap_end as *const u8 as usize };
        info!("Heap: start 0x{:x}, size 0x{:x}", start, end - start);
        unsafe { HEAP.init(start, end - start) }
    }
}

#[cortex_m_rt::entry]
fn main() -> ! {
    #[allow(unused_mut)]
    let mut clock_config = embassy_rp::clocks::ClockConfig::crystal(12_000_000);
    let ps = embassy_rp::init(embassy_rp::config::Config::new(clock_config));

    static EXECUTOR: StaticCell<Executor> = StaticCell::new();
    let executor = EXECUTOR.init(Executor::new());
    executor.run(|spawner| spawner.spawn(main_task(spawner, ps).unwrap()));
}

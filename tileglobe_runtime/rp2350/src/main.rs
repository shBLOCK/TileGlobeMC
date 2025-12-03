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
use embassy_time::Duration;
use embassy_time::Timer;
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
use embassy_net::tcp::TcpSocket;
use embassy_rp::clocks::RoscRng;
use embedded_alloc::LlffHeap as Heap;
use log::warn;
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use tileglobe::world::block::BlockState;
use tileglobe::world::chunk::Chunk;
use tileglobe::world::{ChunkLocalPos, ChunkPos};
use tileglobe::world::world::LocalWorld;
use tileglobe_server::mc_server::MCServer;
use tileglobe_server::MCClient;

#[global_allocator]
static HEAP: Heap = Heap::empty();

type _World = LocalWorld<CriticalSectionRawMutex, -1, -1, 3, 3>;
static WORLD: StaticCell<_World> = StaticCell::new();
static MC_SERVER: StaticCell<MCServer<'_, CriticalSectionRawMutex, _World>> = StaticCell::new();

#[embassy_executor::task(pool_size = 1)]
async fn main_task(spawner: Spawner, ps: Peripherals) -> ! {
    {
        embassy_rp::psram::Psram::new(
            embassy_rp::qmi_cs1::QmiCs1::new(ps.QMI_CS1, ps.PIN_47),
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

    {
        bind_interrupts!(struct Irqs {
            PIO2_IRQ_0 => InterruptHandler<PIO2>;
        });

        #[embassy_executor::task(pool_size = 1)]
        async fn cyw43_task(
            runner: cyw43::Runner<
                'static,
                Output<'static>,
                cyw43_pio::PioSpi<'static, PIO2, 0, DMA_CH0>,
            >,
        ) -> ! {
            runner.run().await
        }

        #[embassy_executor::task(pool_size = 1)]
        async fn net_task(
            mut runner: embassy_net::Runner<'static, cyw43::NetDriver<'static>>,
        ) -> ! {
            runner.run().await
        }

        let fw = include_bytes!("../../../embassy/cyw43-firmware/43439A0.bin");
        let clm = include_bytes!("../../../embassy/cyw43-firmware/43439A0_clm.bin");

        let pwr = Output::new(ps.PIN_23, Level::Low);
        let cs = Output::new(ps.PIN_25, Level::High);
        let mut pio = Pio::new(ps.PIO2, Irqs);
        let spi = cyw43_pio::PioSpi::new(
            &mut pio.common,
            pio.sm0,
            cyw43_pio::DEFAULT_CLOCK_DIVIDER,
            pio.irq0,
            cs,
            ps.PIN_24,
            ps.PIN_29,
            ps.DMA_CH0,
        );

        static STATE: StaticCell<cyw43::State> = StaticCell::new();
        let state = STATE.init(cyw43::State::new());
        let (net_device, mut control, runner) = cyw43::new(state, pwr, spi, fw).await;
        spawner.spawn(unwrap!(cyw43_task(runner)));

        control.init(clm).await;
        control
            .set_power_management(cyw43::PowerManagementMode::None)
            .await;

        // Use a link-local address for communication without DHCP server
        let config = embassy_net::Config::ipv4_static(embassy_net::StaticConfigV4 {
            address: embassy_net::Ipv4Cidr::new(embassy_net::Ipv4Address::new(169, 254, 1, 1), 16),
            dns_servers: heapless::Vec::new(),
            gateway: None,
        });

        // Generate random seed
        let seed = RoscRng.next_u64();

        // Init network stack
        static RESOURCES: StaticCell<embassy_net::StackResources<3>> = StaticCell::new();
        let (stack, runner) = embassy_net::new(
            net_device,
            config,
            RESOURCES.init(embassy_net::StackResources::new()),
            seed,
        );

        spawner.spawn(unwrap!(net_task(runner)));

        control.start_ap_wpa2("TileGlobeMC", "password", 5).await;
        // control.start_ap_open("TileGlobeMC", 5).await;

        let world = WORLD.init(_World::new());

        for cz in -1i16..=1 {
            for cx in -1i16..=1 {
                let mut chunk = Chunk::new(-4..=19);
                for sz in 0..16u8 {
                    for sx in 0..16u8 {
                        for y in (-4 * 16)..(19 * 16i16) {
                            let (x, z) = (cx * 16 + sx as i16, cz * 16 + sz as i16);
                            let mut blockstate = BlockState(0);
                            if (-10..=-1).contains(&y) {
                                blockstate = BlockState(10);
                            }
                            if blockstate.0 != 0 {
                                let _ = chunk.set_block_state(ChunkLocalPos::new(sx, y, sz), blockstate);
                            }
                        }
                    }
                }
                world.set_chunk(ChunkPos::new(cx, cz), chunk).await.unwrap();
            }
        }

        let mc_server = MC_SERVER.init(MCServer::new(world));

        #[embassy_executor::task(pool_size = 3)]
        async fn socket_task(mc_server: &'static MCServer<'static, CriticalSectionRawMutex, _World>, stack: Stack<'static>) -> ! {
            let mut rx_buffer = [0; 4096];
            let mut tx_buffer = [0; 32768];

            loop {
                let mut socket = TcpSocket::new(stack, &mut rx_buffer, &mut tx_buffer);

                if let Err(err) = socket.accept(25565).await {
                    warn!("Failed to accept connection: {:?}", err);
                    continue;
                }

                let endpoint = socket.remote_endpoint();

                info!("Connected to {:?}", endpoint);

                let (mut rx, mut tx) = socket.split();

                let mut client = MCClient::<CriticalSectionRawMutex, _, _, _, _>::new(
                    mc_server,
                    &mut rx,
                    &mut tx,
                    endpoint.map(|ep| SocketAddr::new(ep.addr.into(), ep.port)),
                );
                let result = client.run().await;
                info!("Client disconnected: {:?}", result);

                socket.close();
                if let Err(err) = socket.flush().await {
                    warn!("Failed to close socket: {:?}", err);
                }
                socket.abort();
            }
        }

        for _ in 0..3 {
            spawner.spawn(socket_task(mc_server, stack).unwrap());
        }
    }

    let mut i = 0;
    loop {
        info!("Hello world! {}", i);
        Timer::after_secs(1).await;
        i += 1;
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

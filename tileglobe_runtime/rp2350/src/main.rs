#![no_std]
#![no_main]

extern crate alloc;

use alloc::boxed::Box;
use alloc::vec::Vec;
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
use tileglobe::world::world::{_World, RedstoneOverride};

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
use embassy_rp::spinlock_mutex::SpinlockRawMutex;
use embassy_sync::blocking_mutex::raw::{CriticalSectionRawMutex, NoopRawMutex};
use embassy_sync::mutex::Mutex;
use embedded_alloc::LlffHeap as Heap;
use log::warn;
use tileglobe::world::block::BlockState;
use tileglobe::world::chunk::Chunk;
use tileglobe::world::world::{LocalWorld, World};
use tileglobe_proc_macro::mc_block_id_base;
use tileglobe_server::MCClient;
use tileglobe_server::mc_server::MCServer;
use tileglobe_utils::direction::Direction;
use tileglobe_utils::pos::{BlockPos, ChunkLocalPos, ChunkPos};

#[global_allocator]
static HEAP: Heap = Heap::empty();

static WORLD: StaticCell<_World> = StaticCell::new();
static MC_SERVER: StaticCell<MCServer<'_, SpinlockRawMutex<0>, _World>> = StaticCell::new();

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
    async fn net_task(mut runner: embassy_net::Runner<'static, cyw43::NetDriver<'static>>) -> ! {
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

    control.start_ap_wpa2("TileGlobeMC", "password", 8).await;
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
                            let _ =
                                chunk.set_block_state(ChunkLocalPos::new(sx, y, sz), blockstate);
                        }
                    }
                }
            }
            world.set_chunk(ChunkPos::new(cx, cz), chunk).await.unwrap();
        }
    }

    let mut adc = Adc::new(ps.ADC, AdcIrqs, embassy_rp::adc::Config::default());
    let mut adc0 = embassy_rp::adc::Channel::new_pin(ps.PIN_40, Pull::None);
    let mut adc1 = embassy_rp::adc::Channel::new_pin(ps.PIN_41, Pull::None);
    let mut adc2 = embassy_rp::adc::Channel::new_pin(ps.PIN_42, Pull::None);
    let mut out0 = embassy_rp::gpio::Output::new(ps.PIN_20, Level::Low);
    let mut out1 = embassy_rp::gpio::Output::new(ps.PIN_21, Level::Low);
    let mut out2 = embassy_rp::gpio::Output::new(ps.PIN_22, Level::Low);

    struct GPIORedstoneOverride<'a, const N: usize> {
        adc: Adc<'a, embassy_rp::adc::Async>,
        block_to_adc: [(BlockState, embassy_rp::adc::Channel<'a>); N],
    }
    impl<'a, const N: usize> RedstoneOverride for GPIORedstoneOverride<'a, N> {
        async fn redstone_override(
            &mut self,
            world: &_World,
            pos: BlockPos,
            blockstate: BlockState,
            direction: Direction,
            strong: bool,
        ) -> Option<u8> {
            if let Some((_, channel)) = self
                .block_to_adc
                .iter_mut()
                .find(|(bs, _)| *bs == blockstate)
            {
                return Some((self.adc.read(channel).await.unwrap() / (4096 / 16)) as u8);
            }
            None
        }
    }

    const ADC_BLOCKS: [BlockState; 3] = [
        BlockState(mc_block_id_base!("red_wool")),
        BlockState(mc_block_id_base!("orange_wool")),
        BlockState(mc_block_id_base!("yellow_wool")),
    ];
    let mut dac_blocks = [
        (BlockState(mc_block_id_base!("green_wool")), out0),
        (BlockState(mc_block_id_base!("blue_wool")), out1),
        (BlockState(mc_block_id_base!("black_wool")), out2)
    ];

    world.redstone_override = Some(Mutex::new(Box::new(GPIORedstoneOverride {
        adc,
        block_to_adc: [
            (ADC_BLOCKS[0], adc0),
            (ADC_BLOCKS[1], adc1),
            (ADC_BLOCKS[2], adc2),
        ],
    })));

    let mc_server = MC_SERVER.init(MCServer::new(world));

    #[embassy_executor::task(pool_size = 3)]
    async fn socket_task(
        mc_server: &'static MCServer<'static, SpinlockRawMutex<0>, _World>,
        stack: Stack<'static>,
    ) -> ! {
        let mut rx_buffer = [0u8; 4096];
        let mut tx_buffer = [0u8; 32768];

        loop {
            let mut socket = TcpSocket::new(
                stack,
                unsafe { &mut *(&mut rx_buffer as *mut [u8]) },
                unsafe { &mut *(&mut tx_buffer as *mut [u8]) },
            );

            if let Err(err) = socket.accept(25565).await {
                warn!("Failed to accept connection: {:?}", err);
                continue;
            }

            let endpoint = socket.remote_endpoint();

            info!("Connected to {:?}", endpoint);

            let (mut rx, mut tx) = unsafe { &mut *(&mut socket as *mut TcpSocket) }.split();

            let mut client = MCClient::<SpinlockRawMutex<1>, _, _, _>::new(
                mc_server,
                unsafe { &mut *(&mut rx as *mut TcpReader) },
                unsafe { &mut *(&mut tx as *mut TcpWriter) },
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

    let mut tick_ticker = Ticker::every(Duration::from_hz(20));
    let mut i = 0u32;

    bind_interrupts!(struct AdcIrqs {
        ADC_IRQ_FIFO => embassy_rp::adc::InterruptHandler;
    });

    // let mut adc_temp = embassy_rp::adc::Channel::new_temp_sensor(ps.ADC_TEMP_SENSOR);
    //
    // // let mut samples = [0f32; 20];
    // let mut samples = Vec::<f32>::new();
    //
    // struct RedstoneOverride<'a> {
    //     adc0: embassy_rp::adc::Channel<'a>,
    //     adc1: embassy_rp::adc::Channel<'a>,
    //     adc2: embassy_rp::adc::Channel<'a>,
    // }

    loop {
        info!("Tick {}", i);
        //     let adc_value_1 = adc.read(&mut adc40).await.unwrap() as f32 / 4096.0;
        //     let adc_value_2 = adc.read(&mut adc41).await.unwrap() as f32 / 4096.0;
        //
        //     samples.push(adc_value_1);
        //     if samples.len() > 100 {
        //         samples.remove(0);
        //     }
        //
        //     let adc_temperature = adc.read(&mut adc_temp).await.unwrap() as f32 / 4096.0;
        //     info!("Adc temp: {}", adc_temperature);
        //     for y in 0i16..32 {
        //         for x in 0i16..32 {
        //             let ind = samples.len() as i16 - 1 - x;
        //             let adc_value_1 = if ind >= 0 {
        //                 let a = *samples.get(ind as usize).or(Some(&0f32)).unwrap();
        //                 a
        //             } else {
        //                 adc_value_1
        //             };
        //             let state = if y as f32 / 32.0 <= adc_value_1 && x as f32 / 32.0 <= adc_value_2 {
        //                 BlockState(if adc_temperature < 0.19 { 5958 } else if adc_temperature < 0.2 { 86 + 15 } else if adc_temperature < 0.205 { 117 } else { 4340 })
        //             } else {
        //                 BlockState(0)
        //             };
        //             // if x > 10 && x < 16 {
        //             let _ = world.set_block_state(BlockPos::new(x, y, 0), state).await;
        //             // }
        //         }
        //         embassy_futures::yield_now().await;
        //         // Timer::after_micros(1).await;
        //     }
        //
        //     // for y in 0..32 {
        //     //     let state = if y as f32 / 32.0 < adc_value_1 {
        //     //         BlockState(4340)
        //     //     } else {
        //     //         BlockState(0)
        //     //     };
        //     //     world
        //     //         .set_block_state(BlockPos::new(0, y, 0), state)
        //     //         .await
        //     //         .unwrap();
        //     // }

        // let adc_value_0 =

        for x in -16i16..32i16 {
            let z = -16i16;
            let pos = BlockPos::new(x, 0, z);
            if let Ok(blockstate) = world.get_block_state(pos).await {
                if ADC_BLOCKS.contains(&blockstate) {
                    world.update_neighbors(pos).await;
                }
                if let Some((_, out)) = dac_blocks.iter_mut().find(|(bs, _)| *bs == blockstate) {
                    let signal = world.get_signal_to(pos).await;
                    out.set_level((signal > 0).into());
                }
            }
        }
        embassy_futures::yield_now().await; // important! or else wifi dies..?
        world.tick().await;
        embassy_futures::yield_now().await; // important! or else wifi dies..?
        mc_server.tick().await;
        i += 1;
        tick_ticker.next().await;
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

#![no_std]
#![no_main]

use tileglobe::world::world::World;
extern crate alloc;

use tileglobe_server::MCClient;
use tileglobe_proc_macro::mc_block_id_base;
use tileglobe_server::mc_server::MCServer;
use tileglobe::world::chunk::Chunk;
use tileglobe_utils::pos::ChunkLocalPos;
use tileglobe_utils::pos::ChunkPos;
use tileglobe::world::world::RedstoneOverride;
use tileglobe_utils::pos::BlockPos;
use tileglobe::world::world::_World;
use tileglobe::world::block::BlockState;
use alloc::boxed::Box;
use core::net::SocketAddr;
use defmt::*;
use embassy_executor::Executor;
use embassy_executor::Spawner;
use embassy_rp::gpio::Output;
use embassy_rp::gpio::{Input, Level, Pull};
use embassy_rp::peripherals::{DMA_CH0, DMA_CH15, PIO2, SPI1};
use embassy_rp::pio::InterruptHandler;
use embassy_rp::pio::Pio;
use embassy_rp::{Peripherals, bind_interrupts};
use embassy_time::{Delay, Duration, Instant, Ticker, Timer};
use embedded_hal_bus::spi::ExclusiveDevice;
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

use embassy_net::tcp::{TcpReader, TcpSocket, TcpWriter};
use embassy_net::{Stack, StackResources};
use embassy_rp::adc::Adc;
use embassy_rp::clocks::RoscRng;
use embassy_rp::spinlock_mutex::SpinlockRawMutex;
use embassy_sync::blocking_mutex::raw::{CriticalSectionRawMutex, NoopRawMutex};
use embassy_sync::mutex::Mutex;
use embedded_alloc::LlffHeap as Heap;

#[global_allocator]
static HEAP: Heap = Heap::empty();

#[embassy_executor::task(pool_size = 1)]
async fn main_task(spawner: Spawner, ps: Peripherals) -> ! {
    {
        let mut cfg = embassy_rp::psram::Config::aps6404l();
        cfg.clock_hz = 300_000_000;
        cfg.max_mem_freq = 144_000_000;
        embassy_rp::psram::Psram::new(
            embassy_rp::qmi_cs1::QmiCs1::new(ps.QMI_CS1, ps.PIN_47),
            cfg,
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

    let net_stack = {
        let miso = ps.PIN_28;
        let mosi = ps.PIN_31;
        let sck = ps.PIN_30;
        let cs = Output::new(ps.PIN_46, Level::High);
        let handshake = Input::new(ps.PIN_3, Pull::Up);
        let ready = Input::new(ps.PIN_23, Pull::None);
        let reset = Output::new(ps.PIN_22, Level::Low);

        let mut spi_cfg = embassy_rp::spi::Config::default();
        spi_cfg.frequency = 28_000_000;
        spi_cfg.polarity = embassy_rp::spi::Polarity::IdleHigh;
        spi_cfg.phase = embassy_rp::spi::Phase::CaptureOnSecondTransition;
        let spi =
            embassy_rp::spi::Spi::new(ps.SPI1, sck, mosi, miso, ps.DMA_CH0, ps.DMA_CH1, spi_cfg);
        let spi = ExclusiveDevice::new(spi, cs, Delay).unwrap();

        let interface = embassy_net_esp_hosted::SpiInterface::new(spi, handshake, ready);

        static ESP_STATE: StaticCell<embassy_net_esp_hosted::State> = StaticCell::new();
        let (device, mut control, runner) = embassy_net_esp_hosted::new(
            ESP_STATE.init(embassy_net_esp_hosted::State::new()),
            interface,
            reset,
        )
        .await;

        #[embassy_executor::task(pool_size = 1)]
        async fn wifi_task(
            runner: embassy_net_esp_hosted::Runner<
                'static,
                embassy_net_esp_hosted::SpiInterface<
                    ExclusiveDevice<
                        embassy_rp::spi::Spi<'static, SPI1, embassy_rp::spi::Async>,
                        Output<'static>,
                        Delay,
                    >,
                    Input<'static>,
                >,
                Output<'static>,
            >,
        ) -> ! {
            runner.run().await
        }
        spawner.spawn(unwrap!(wifi_task(runner)));

        unwrap!(control.init().await);
        while let Err(err) = control
            .connect("ME Wireless Access Point", "82259530")
            .await
        {
            warn!("Failed to connect to AP: {:?}", err);
        }

        let config = embassy_net::Config::dhcpv4(Default::default());

        let seed = RoscRng.next_u64();

        // Init network stack
        static RESOURCES: StaticCell<StackResources<4>> = StaticCell::new();
        let (stack, runner) =
            embassy_net::new(device, config, RESOURCES.init(StackResources::new()), seed);

        #[embassy_executor::task(pool_size = 1)]
        async fn net_task(
            mut runner: embassy_net::Runner<'static, embassy_net_esp_hosted::NetDriver<'static>>,
        ) -> ! {
            runner.run().await
        }
        spawner.spawn(unwrap!(net_task(runner)));

        stack.wait_link_up().await;
        info!("Link up!");

        stack
    };

    static WORLD: StaticCell<_World> = StaticCell::new();
    static MC_SERVER: StaticCell<MCServer<'_, SpinlockRawMutex<0>, _World>> = StaticCell::new();

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
    let mut adc0 = embassy_rp::adc::Channel::new_pin(ps.PIN_41, Pull::None);
    let mut adc1 = embassy_rp::adc::Channel::new_pin(ps.PIN_42, Pull::None);
    let mut adc2 = embassy_rp::adc::Channel::new_pin(ps.PIN_43, Pull::None);
    let mut adc3 = embassy_rp::adc::Channel::new_pin(ps.PIN_44, Pull::None);
    let mut adc4 = embassy_rp::adc::Channel::new_pin(ps.PIN_45, Pull::None);
    let mut adc_temp = embassy_rp::adc::Channel::new_temp_sensor(ps.ADC_TEMP_SENSOR);
    let mut out0 = embassy_rp::gpio::Output::new(ps.PIN_6, Level::Low);
    let mut out1 = embassy_rp::gpio::Output::new(ps.PIN_7, Level::Low);
    let mut out2 = embassy_rp::gpio::Output::new(ps.PIN_8, Level::Low);
    let mut out3 = embassy_rp::gpio::Output::new(ps.PIN_9, Level::Low);
    let mut out4 = embassy_rp::gpio::Output::new(ps.PIN_10, Level::Low);

    struct GPIORedstoneOverride<'a, const N: usize> {
        adc: Adc<'a, embassy_rp::adc::Async>,
        block_to_adc: [(
            BlockState,
            embassy_rp::adc::Channel<'a>,
            Box<dyn Fn(u16) -> u8>,
        ); N],
    }
    impl<'a, const N: usize> RedstoneOverride for GPIORedstoneOverride<'a, N> {
        async fn redstone_override(
            &mut self,
            world: &_World,
            pos: BlockPos,
            blockstate: BlockState,
            direction: tileglobe_utils::direction::Direction,
            strong: bool,
        ) -> Option<u8> {
            if let Some((_, channel, mapping)) = self
                .block_to_adc
                .iter_mut()
                .find(|(bs, _, _)| *bs == blockstate)
            {
                return Some(mapping(self.adc.read(channel).await.unwrap()));
            }
            None
        }
    }

    const ADC_BLOCKS: [BlockState; 4] = [
        BlockState(mc_block_id_base!("white_wool")),
        BlockState(mc_block_id_base!("black_wool")),
        BlockState(mc_block_id_base!("orange_wool")),
        BlockState(mc_block_id_base!("magenta_wool")),
    ];
    let mut dac_blocks = [
        (BlockState(mc_block_id_base!("red_wool")), out0),
        (BlockState(mc_block_id_base!("yellow_wool")), out1),
        (BlockState(mc_block_id_base!("green_wool")), out2),
        (BlockState(mc_block_id_base!("blue_wool")), out3),
    ];

    world.redstone_override = Some(Mutex::new(Box::new(GPIORedstoneOverride {
        adc,
        block_to_adc: [
            (ADC_BLOCKS[0], adc0, Box::new(|t| (t / (4096 / 16)) as u8)),
            (ADC_BLOCKS[1], adc1, Box::new(|t| (t / (4096 / 16)) as u8)),
            (ADC_BLOCKS[2], adc2, Box::new(|t| (t / (4096 / 16)) as u8)),
            (
                ADC_BLOCKS[3],
                adc_temp,
                Box::new(|t| ((870 - t as i16) / 3).clamp(0, 15) as u8),
            ),
        ],
    })));

    let mc_server = MC_SERVER.init(MCServer::new(world));

    #[embassy_executor::task(pool_size = 3)]
    async fn socket_task(
        mc_server: &'static MCServer<'static, SpinlockRawMutex<0>, _World>,
        net_stack: Stack<'static>,
    ) -> ! {
        let mut rx_buffer = [0u8; 4096];
        let mut tx_buffer = [0u8; 4096];

        loop {
            let mut socket = TcpSocket::new(
                net_stack,
                unsafe { &mut *(&mut rx_buffer as *mut [u8]) },
                unsafe { &mut *(&mut tx_buffer as *mut [u8]) },
            );
            socket.set_nagle_enabled(false);

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
        spawner.spawn(socket_task(mc_server, net_stack).unwrap());
    }

    let mut tick_ticker = Ticker::every(Duration::from_hz(20));
    let mut i = 0u32;

    bind_interrupts!(struct AdcIrqs {
        ADC_IRQ_FIFO => embassy_rp::adc::InterruptHandler;
    });

    loop {
        info!("Tick {}", i);

        let st = Instant::now();
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
        info!("GPIO: {}", Instant::now() - st);

        embassy_futures::yield_now().await; // important! or else wifi dies..?

        let st = Instant::now();
        world.tick().await;
        info!("World Tick: {}", Instant::now() - st);

        embassy_futures::yield_now().await; // important! or else wifi dies..?

        let st = Instant::now();
        mc_server.tick().await;
        info!("Server Tick: {}", Instant::now() - st);

        i += 1;
        tick_ticker.next().await;
    }

    loop {
        info!("Hello world!");
        Timer::after_millis(500).await;
    }
}

#[cortex_m_rt::entry]
fn main() -> ! {
    let clock_config = embassy_rp::clocks::ClockConfig::system_freq(300_000_000).unwrap();
    // let clock_config = embassy_rp::clocks::ClockConfig::system_freq(150_000_000).unwrap();
    let ps = embassy_rp::init(embassy_rp::config::Config::new(clock_config));

    static EXECUTOR: StaticCell<Executor> = StaticCell::new();
    let executor = EXECUTOR.init(Executor::new());
    executor.run(|spawner| spawner.spawn(main_task(spawner, ps).unwrap()));
}

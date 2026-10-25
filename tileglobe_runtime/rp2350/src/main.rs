#![no_std]
#![no_main]

extern crate alloc;

use alloc::boxed::Box;
use alloc::vec::Vec;
use core::net::{Ipv4Addr, SocketAddr};
use defmt::*;
use embassy_executor::Executor;
use embassy_executor::Spawner;
use embassy_rp::gpio::Output;
use embassy_rp::gpio::{Input, Level, Pull};
use embassy_rp::peripherals::{DMA_CH0, DMA_CH15, PIO2};
use embassy_rp::pio::Pio;
use embassy_rp::{Peripherals, bind_interrupts};
use embassy_time::{Duration, Instant, Ticker, Timer};
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

use embassy_net::iface::dhcpv4_server::DhcpServerConfig;
use embassy_net::tcp::{TcpListener, TcpReader, TcpSocket, TcpWriter};
use embassy_net::wire::{IpCidr, Ipv4Cidr};
use embassy_net::{Stack, StaticPool};
use embassy_rp::adc::Adc;
use embassy_rp::clocks::{CoreVoltage, RoscRng};
use embassy_rp::spinlock_mutex::SpinlockRawMutex;
use embassy_sync::blocking_mutex::raw::{CriticalSectionRawMutex, NoopRawMutex};
use embassy_sync::mutex::Mutex;
use embedded_alloc::TlsfHeap as Heap;
use log::warn;
use tileglobe::utils::Direction;
use tileglobe::utils::{BlockPos, ChunkLocalPos, ChunkPos};
use tileglobe::world::block::BlockState;
use tileglobe::world::chunk::Chunk;
use tileglobe::world::utils::mc_block_index_base;
use tileglobe::world::world::{LocalWorld, World};
use tileglobe_server::MCClient;
use tileglobe_server::mc_client::TMP_TICK_RATE;
use tileglobe_server::mc_server::MCServer;

#[global_allocator]
static HEAP: Heap = Heap::empty();

static WORLD: StaticCell<_World> = StaticCell::new();
static MC_SERVER: StaticCell<MCServer<'_, SpinlockRawMutex<0>, _World>> = StaticCell::new();

const SYS_CLK_FREQ: u32 = 290_000_000;

#[cfg(feature = "net_cyw43")]
mod setup_net {
    use cyw43::aligned_bytes;
    use cyw43_pio::PioSpi;
    use defmt::unwrap;
    use embassy_executor::Spawner;
    use embassy_net::StaticPool;
    use embassy_rp::gpio::{Level, Output};
    use embassy_rp::peripherals::{DMA_CH0, DMA_CH1, PIN_23, PIN_24, PIN_25, PIN_29, PIO2};
    use embassy_rp::pio::Pio;
    use embassy_rp::{Peri, bind_interrupts};
    use static_cell::StaticCell;

    pub(super) type NetDriver<'a> = cyw43::NetDriver<'a>;

    pub(super) static POOL: StaticPool<{ cyw43::RX_BUF_SIZE }, 16, { cyw43::RX_BUF_ALIGN }> =
        StaticPool::new();

    pub(super) async fn _setup_net(
        spawner: &Spawner,
        pwr: Peri<'static, PIN_23>,
        cs: Peri<'static, PIN_25>,
        pio: Peri<'static, PIO2>,
        dio: Peri<'static, PIN_24>,
        clk: Peri<'static, PIN_29>,
        dma_tx: Peri<'static, DMA_CH0>,
        dma_rx: Peri<'static, DMA_CH1>,
    ) -> NetDriver<'static> {
        bind_interrupts!(struct Irqs {
            PIO2_IRQ_0 => embassy_rp::pio::InterruptHandler<PIO2>;
            DMA_IRQ_0 => embassy_rp::dma::InterruptHandler<DMA_CH0>, embassy_rp::dma::InterruptHandler<DMA_CH1>;
        });

        #[embassy_executor::task(pool_size = 1)]
        async fn cyw43_task(
            runner: cyw43::Runner<
                'static,
                cyw43::SpiBus<Output<'static>, PioSpi<'static, PIO2, 0>>,
                cyw43::Cyw43439,
            >,
        ) -> ! {
            runner.run().await
        }

        let fw = aligned_bytes!("../../../embassy/cyw43-firmware/43439A0.bin");
        let clm = aligned_bytes!("../../../embassy/cyw43-firmware/43439A0_clm.bin");
        let nvram = aligned_bytes!("../../../embassy/cyw43-firmware/nvram_rp2040.bin");

        let pwr = Output::new(pwr, Level::Low);
        let cs = Output::new(cs, Level::High);
        let mut pio = Pio::new(pio, Irqs);
        let spi = cyw43_pio::PioSpi::new(
            &mut pio.common,
            pio.sm0,
            cyw43_pio::RM2_CLOCK_DIVIDER,
            pio.irq0,
            cs,
            dio,
            clk,
            embassy_rp::dma::Channel::new(dma_tx, Irqs),
            embassy_rp::dma::Channel::new(dma_rx, Irqs),
        );

        static STATE: StaticCell<cyw43::State> = StaticCell::new();
        let state = STATE.init(cyw43::State::new());
        let (net_device, mut control, runner) = cyw43::new(state, pwr, spi, fw, nvram).await;
        spawner.spawn(unwrap!(cyw43_task(runner)));

        control.init(clm).await;
        control
            .set_power_management(cyw43::PowerManagementMode::None)
            .await;

        control.start_ap_wpa2("TileGlobeMC", "password", 8).await;

        net_device
    }

    macro_rules! setup_net {
        ($spawner: expr, $ps: expr) => {
            $crate::setup_net::_setup_net(
                $spawner,
                $ps.PIN_23,
                $ps.PIN_25,
                $ps.PIO2,
                $ps.PIN_24,
                $ps.PIN_29,
                $ps.DMA_CH0,
                $ps.DMA_CH1,
            )
        };
    }
    pub(super) use setup_net;
}

#[cfg(feature = "net_usb")]
mod setup_net {
    use defmt::unwrap;
    use embassy_executor::Spawner;
    use embassy_net::StaticPool;
    use embassy_rp::peripherals::USB;
    use embassy_rp::usb::Driver;
    use embassy_rp::{Peri, bind_interrupts};
    use embassy_usb::class::cdc_ncm::device::embassy_net::{Runner, State as NetState};
    use embassy_usb::class::cdc_ncm::device::{CdcNcmClass, State};
    use embassy_usb::{Builder, Config, UsbDevice};
    use static_cell::StaticCell;

    pub(super) type NetDriver<'a> = embassy_usb::class::cdc_ncm::device::embassy_net::Device<'a>;

    pub(super) static POOL: StaticPool = StaticPool::new();

    pub(super) async fn _setup_net(
        spawner: &Spawner,
        usb: Peri<'static, USB>,
    ) -> NetDriver<'static> {
        bind_interrupts!(struct Irqs {
            USBCTRL_IRQ => embassy_rp::usb::InterruptHandler<USB>;
        });

        let driver = Driver::new(usb, Irqs);

        let mut config = Config::new(0xc0de, 0xcafe);
        config.manufacturer = Some("shBLOCK");
        config.product = Some("TileGlobeMC");
        config.serial_number = Some("12345678");
        config.max_power = 500;
        config.max_packet_size_0 = 64;

        // Create embassy-usb DeviceBuilder using the driver and config.
        static CONFIG_DESC: StaticCell<[u8; 256]> = StaticCell::new();
        static BOS_DESC: StaticCell<[u8; 256]> = StaticCell::new();
        static CONTROL_BUF: StaticCell<[u8; 128]> = StaticCell::new();
        let mut builder = Builder::new(
            driver,
            config,
            &mut CONFIG_DESC.init([0; 256])[..],
            &mut BOS_DESC.init([0; 256])[..],
            &mut [], // no msos descriptors
            &mut CONTROL_BUF.init([0; 128])[..],
        );

        // Our MAC addr.
        let our_mac_addr = [0xCC, 0xCC, 0xCC, 0xCC, 0xCC, 0xCC];
        // Host's MAC addr. This is the MAC the host "thinks" its USB-to-ethernet adapter has.
        let host_mac_addr = [0x88, 0x88, 0x88, 0x88, 0x88, 0x88];

        // Create classes on the builder.
        static STATE: StaticCell<State> = StaticCell::new();
        let class = CdcNcmClass::new(&mut builder, STATE.init(State::new()), host_mac_addr, 64);

        // Build the builder.
        let usb = builder.build();

        #[embassy_executor::task]
        async fn usb_task(mut device: UsbDevice<'static, Driver<'static, USB>>) -> ! {
            device.run().await
        }
        spawner.spawn(unwrap!(usb_task(usb)));

        static NET_STATE: StaticCell<NetState<4, 4>> = StaticCell::new();
        let (runner, device) = class.into_embassy_net_device::<4, 4>(
            NET_STATE.init(NetState::new()),
            our_mac_addr,
            1514,
        );

        #[embassy_executor::task]
        async fn usb_ncm_task(class: Runner<'static, Driver<'static, USB>>) -> ! {
            class.run().await
        }
        spawner.spawn(unwrap!(usb_ncm_task(runner)));

        device
    }

    macro_rules! setup_net {
        ($spawner: expr, $ps: expr) => {
            $crate::setup_net::_setup_net($spawner, $ps.USB)
        };
    }
    pub(super) use setup_net;
}

#[embassy_executor::task(pool_size = 1)]
async fn main_task(spawner: Spawner, ps: Peripherals) -> ! {
    {
        let mut cfg = embassy_rp::psram::Config::aps6404l();
        cfg.clock_hz = SYS_CLK_FREQ;
        cfg.max_mem_freq = 144_000_000;
        embassy_rp::psram::Psram::new(embassy_rp::qmi_cs1::QmiCs1::new(ps.QMI_CS1, ps.PIN_47), cfg)
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

    let net_device = setup_net::setup_net!(&spawner, ps).await;

    // Generate random seed
    let seed = RoscRng.next_u64();

    // Init network stack
    static STACK_STORAGE: StaticCell<embassy_net::StackStorage> = StaticCell::new();
    let (stack, runner) = embassy_net::Stack::new(
        STACK_STORAGE.init(embassy_net::StackStorage::new()),
        &setup_net::POOL,
        seed,
    );
    static DEVICE: StaticCell<setup_net::NetDriver<'static>> = StaticCell::new();
    let iface = stack.add_iface_borrowed(DEVICE.init(net_device)).unwrap();
    // iface
    //     .add_ip_addr(IpCidr::new(Ipv4Addr::new(169, 254, 1, 1).into(), 16))
    //     .unwrap();
    iface
        .add_ip_addr(IpCidr::new(Ipv4Addr::new(192, 168, 1, 1).into(), 24))
        .unwrap();
    let dhcp_config = DhcpServerConfig::new(
        Ipv4Addr::new(192, 168, 1, 2),
        Ipv4Addr::new(192, 168, 1, 255),
    );
    iface.set_dhcpv4_server(Some(dhcp_config)).unwrap();

    #[embassy_executor::task(pool_size = 1)]
    async fn net_task(mut runner: embassy_net::Runner<'static>) -> ! {
        runner.run().await
    }

    spawner.spawn(unwrap!(net_task(runner)));

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
    let mut adc_temp = embassy_rp::adc::Channel::new_temp_sensor(ps.ADC_TEMP_SENSOR);
    let mut out0 = embassy_rp::gpio::Output::new(ps.PIN_19, Level::Low);
    let mut out1 = embassy_rp::gpio::Output::new(ps.PIN_20, Level::Low);
    let mut out2 = embassy_rp::gpio::Output::new(ps.PIN_21, Level::Low);
    let mut out3 = embassy_rp::gpio::Output::new(ps.PIN_22, Level::Low);

    struct GPIORedstoneOverride<'a, const N: usize> {
        adc: Adc<'a, embassy_rp::mode::Async>,
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
            direction: Direction,
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
        BlockState(mc_block_index_base!("white_wool")),
        BlockState(mc_block_index_base!("black_wool")),
        BlockState(mc_block_index_base!("orange_wool")),
        BlockState(mc_block_index_base!("magenta_wool")),
    ];
    let mut dac_blocks = [
        (BlockState(mc_block_index_base!("red_wool")), out0),
        (BlockState(mc_block_index_base!("yellow_wool")), out1),
        (BlockState(mc_block_index_base!("green_wool")), out2),
        (BlockState(mc_block_index_base!("blue_wool")), out3),
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
        stack: Stack<'static>,
    ) -> ! {
        let mut rx_buffer = [0u8; 4096];
        let mut tx_buffer = [0u8; 4096];

        let mut listener = TcpListener::new(stack).unwrap();
        listener.listen(25565).unwrap();

        loop {
            let accept_token = match listener.accept().await {
                Ok(token) => token,
                Err(err) => {
                    warn!("Failed to accept connection: {:?}", err);
                    continue;
                }
            };

            let mut socket = TcpSocket::new(
                stack,
                unsafe { &mut *(&mut rx_buffer as *mut [u8]) },
                unsafe { &mut *(&mut tx_buffer as *mut [u8]) },
            )
            .unwrap();

            if let Err(err) = socket.accept(accept_token).await {
                warn!("Failed to accept connection: {:?}", err);
                continue;
            }

            let remote_addr = socket.remote_addr();

            info!("Connected to {:?}", remote_addr);

            let (mut rx, mut tx) = unsafe { &mut *(&mut socket as *mut TcpSocket) }.split();

            let mut client = MCClient::<SpinlockRawMutex<1>, _, _, _>::new(
                mc_server,
                unsafe { &mut *(&mut rx as *mut TcpReader) },
                unsafe { &mut *(&mut tx as *mut TcpWriter) },
                remote_addr.map(|ep| SocketAddr::new(ep.addr.into(), ep.port)),
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

    for _ in 0..1 {
        spawner.spawn(socket_task(mc_server, stack).unwrap());
    }

    let mut tick_rate = unsafe { TMP_TICK_RATE };
    let mut tick_ticker = Ticker::every(Duration::from_hz(tick_rate as u64));
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

    info!("Starting tick loop");
    loop {
        // info!("Tick {}", i);
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
        // info!("GPIO: {}", Instant::now() - st);

        embassy_futures::yield_now().await; // important! or else wifi dies..?

        let st = Instant::now();
        world.tick().await;
        // info!("World Tick: {}", Instant::now() - st);

        embassy_futures::yield_now().await; // important! or else wifi dies..?

        let st = Instant::now();
        mc_server.tick().await;
        // info!("Server Tick: {}", Instant::now() - st);

        i += 1;

        if tick_rate != unsafe { TMP_TICK_RATE } {
            tick_rate = unsafe { TMP_TICK_RATE };
            tick_ticker = Ticker::every(Duration::from_hz(tick_rate as u64));
        }
        tick_ticker.next().await;
    }
}

#[cortex_m_rt::entry]
fn main() -> ! {
    #[allow(unused_mut)]
    // let mut clock_config = embassy_rp::clocks::ClockConfig::crystal(12_000_000);
    let mut clock_config = embassy_rp::clocks::ClockConfig::system_freq(SYS_CLK_FREQ).unwrap();
    clock_config.core_voltage = CoreVoltage::V1_30;
    let ps = embassy_rp::init(embassy_rp::config::Config::new(clock_config));

    static EXECUTOR: StaticCell<Executor> = StaticCell::new();
    let executor = EXECUTOR.init(Executor::new());
    executor.run(|spawner| spawner.spawn(main_task(spawner, ps).unwrap()));
}

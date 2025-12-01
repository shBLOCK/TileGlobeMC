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
use tileglobe::master_node::MCClient;

#[global_allocator]
static HEAP: Heap = Heap::empty();

#[embassy_executor::task]
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

    // {
    //     use embassy_net;
    //     use embassy_net_esp_hosted as hosted;
    //
    //     #[embassy_executor::task]
    //     async fn wifi_task(
    //         runner: hosted::Runner<
    //             'static,
    //             hosted::SpiInterface<
    //                 ExclusiveDevice<Spim<'static>, Output<'static>, Delay>,
    //                 Input<'static>,
    //             >,
    //             Output<'static>,
    //         >,
    //     ) -> ! {
    //         runner.run().await
    //     }
    //
    //     #[embassy_executor::task]
    //     async fn net_task(
    //         mut runner: embassy_net::Runner<'static, hosted::NetDriver<'static>>,
    //     ) -> ! {
    //         runner.run().await
    //     }
    //
    //     let miso = ps.PIN_28;
    //     let sck = ps.PIN_30;
    //     let mosi = ps.PIN_31;
    //     let cs = Output::new(ps.PIN_46, Level::High);
    //     let handshake = Input::new(ps.PIN_23, Pull::Up);
    //     let ready = Input::new(ps.PIN_3, Pull::None);
    //     let reset = Output::new(ps.PIN_22, Level::Low);
    //
    //     let config = embassy_rp::spi::Config {
    //         frequency: 32_000_000,
    //         phase: embassy_rp::spi::Phase::CaptureOnSecondTransition,
    //         polarity: embassy_rp::spi::Polarity::IdleLow,
    //     };
    //     // config.mode = spim::MODE_2; // !!!
    //     let spi = embassy_rp::spi::Spi::new(p.SPI3, Irqs, sck, miso, mosi, config);
    //     let spi = embassy_rp::spi::Spi::new(ps.SPI1, , mosi, miso, );
    //     let spi = ExclusiveDevice::new(spi, cs, Delay);
    //
    //     let iface = hosted::SpiInterface::new(spi, handshake, ready);
    //
    //     static ESP_STATE: StaticCell<embassy_net_esp_hosted::State> = StaticCell::new();
    //     let (device, mut control, runner) = embassy_net_esp_hosted::new(
    //         ESP_STATE.init(embassy_net_esp_hosted::State::new()),
    //         iface,
    //         reset,
    //     )
    //     .await;
    //
    //     spawner.spawn(unwrap!(wifi_task(runner)));
    //
    //     unwrap!(control.init().await);
    //     // unwrap!(control.connect(WIFI_NETWORK, WIFI_PASSWORD).await);
    //
    //     // let config = embassy_net::Config::dhcpv4(Default::default());
    //
    //     // // Generate random seed
    //     // let mut rng = Rng::new(p.RNG, Irqs);
    //     // let mut seed = [0; 8];
    //     // rng.blocking_fill_bytes(&mut seed);
    //     // let seed = u64::from_le_bytes(seed);
    //     //
    //     // // Init network stack
    //     // static RESOURCES: StaticCell<StackResources<3>> = StaticCell::new();
    //     // let (stack, runner) = embassy_net::new(device, config, RESOURCES.init(StackResources::new()), seed);
    //
    //     // spawner.spawn(unwrap!(net_task(runner)));
    // }

    {
        bind_interrupts!(struct Irqs {
            PIO2_IRQ_0 => InterruptHandler<PIO2>;
        });

        #[embassy_executor::task]
        async fn cyw43_task(
            runner: cyw43::Runner<
                'static,
                Output<'static>,
                cyw43_pio::PioSpi<'static, PIO2, 0, DMA_CH0>,
            >,
        ) -> ! {
            runner.run().await
        }

        #[embassy_executor::task]
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
            .set_power_management(cyw43::PowerManagementMode::PowerSave)
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

        #[embassy_executor::task(pool_size = 3)]
        async fn socket_task(stack: Stack<'static>) -> ! {
            let mut rx_buffer = [0; 4096];
            let mut tx_buffer = [0; 4096];

            loop {
                let mut socket = TcpSocket::new(stack, &mut rx_buffer, &mut tx_buffer);

                if let Err(err) = socket.accept(25565).await {
                    warn!("Failed to accept connection: {:?}", err);
                    continue;
                }

                let endpoint = socket.remote_endpoint();

                info!("Connected to {:?}", endpoint);

                let mut client = MCClient::new(
                    &mut socket,
                    endpoint.map(|ep| SocketAddr::new(ep.addr.into(), ep.port)),
                );
                client._main_task().await;
                socket.close();
                if let Err(err) = socket.flush().await {
                    warn!("Failed to close socket: {:?}", err);
                }
                socket.abort();
            }
        }

        for _ in 0..3 {
            spawner.spawn(socket_task(stack).unwrap());
        }
    }

    loop {
        info!("Hello world!");
        Timer::after_secs(1).await;
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

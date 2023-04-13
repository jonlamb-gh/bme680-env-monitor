//#![deny(warnings, clippy::all)]
#![no_main]
#![no_std]

mod config;
mod logger;
mod net;
mod panic_handler;
mod reset_reason;
mod tasks;
mod util;

pub mod built_info {
    include!(concat!(env!("OUT_DIR"), "/built.rs"));
}

#[rtic::app(device = stm32f4xx_hal::pac, dispatchers = [EXTI0, EXTI1, EXTI2])]
mod app {
    use crate::net::{Eth, EthernetStorage, NetworkStorage, UdpSocketStorage};
    use crate::reset_reason::ResetReason;
    use crate::tasks::{
        eth_gpio_interrupt_handler_task, ipstack_clock_timer_task, ipstack_poll_task,
        ipstack_poll_timer_task, watchdog_task,
    };
    use crate::{config, util};
    use log::{debug, info};
    use smoltcp::{
        iface::{Config, Interface, SocketHandle, SocketSet},
        socket::udp::{PacketBuffer as UdpPacketBuffer, Socket as UdpSocket},
        wire::EthernetAddress,
    };
    use stm32f4xx_hal::{
        flash::{flash_sectors, FlashExt},
        gpio::{Edge, Input, Output, PushPull, Speed as GpioSpeed, PA0, PC13},
        pac::{self, TIM3},
        prelude::*,
        spi::Spi,
        timer::counter::CounterHz,
        timer::{Event, MonoTimerUs, SysCounterUs, SysEvent},
        watchdog::IndependentWatchdog,
    };

    type OnBoardButtonPin = PA0<Input>;
    type LedPin = PC13<Output<PushPull>>;

    #[shared]
    struct Shared {
        #[lock_free]
        eth: Eth<'static>,
        #[lock_free]
        net: Interface,
        #[lock_free]
        sockets: SocketSet<'static>,
        #[lock_free]
        udp_socket: SocketHandle,
    }

    #[local]
    struct Local {
        net_clock_timer: SysCounterUs,
        ipstack_poll_timer: CounterHz<TIM3>,
        led: LedPin,
        watchdog: IndependentWatchdog,
        button: OnBoardButtonPin,
    }

    /// TIM2 is a 32-bit timer, defaults to having the highest interrupt priority
    #[monotonic(binds = TIM2, default = true)]
    type MicrosecMono = MonoTimerUs<pac::TIM2>;

    #[init(local = [
        eth_storage: EthernetStorage<{Eth::MTU}> = EthernetStorage::new(),
        net_storage: NetworkStorage<1> = NetworkStorage::new(),
        udp_socket_storage: UdpSocketStorage<{config::SOCKET_BUFFER_LEN}> = UdpSocketStorage::new(),
    ])]
    fn init(mut ctx: init::Context) -> (Shared, Local, init::Monotonics) {
        let reset_reason = ResetReason::read_and_clear(&mut ctx.device.RCC);

        let mut syscfg = ctx.device.SYSCFG.constrain();
        let rcc = ctx.device.RCC.constrain();
        let clocks = rcc.cfgr.use_hse(25.MHz()).sysclk(64.MHz()).freeze();

        let mut watchdog = IndependentWatchdog::new(ctx.device.IWDG);
        watchdog.start(config::WATCHDOG_RESET_PERIOD_MS.millis());
        watchdog.feed();

        let gpioa = ctx.device.GPIOA.split();
        let gpiob = ctx.device.GPIOB.split();
        let gpioc = ctx.device.GPIOC.split();

        // Turn it off, active-low
        let led: LedPin = gpioc.pc13.into_push_pull_output_in_state(true.into());

        let button: OnBoardButtonPin = gpioa.pa0.into_pull_up_input();

        // Setup logging impl via USART6, Rx on PA12, Tx on PA11
        // This is also the virtual com port on the nucleo boards: stty -F /dev/ttyACM0 115200
        let log_tx_pin = gpioa.pa11.into_alternate();
        let log_tx = ctx
            .device
            .USART6
            .tx(log_tx_pin, 115_200.bps(), &clocks)
            .unwrap();
        unsafe { crate::logger::init_logging(log_tx) };

        debug!("Watchdog: inerval {}", watchdog.interval());

        info!("############################################################");
        info!(
            "{} {} ({})",
            crate::built_info::PKG_NAME,
            config::FIRMWARE_VERSION,
            crate::built_info::PROFILE
        );
        info!("Build date: {}", crate::built_info::BUILT_TIME_UTC);
        info!("{}", crate::built_info::RUSTC_VERSION);
        if let Some(gc) = crate::built_info::GIT_COMMIT_HASH {
            info!("git commit: {}", gc);
        }
        info!("Serial number: {:X}", util::read_device_serial_number());
        info!(
            "Device ID: 0x{:X} ({})",
            config::DEVICE_ID,
            config::DEVICE_ID
        );
        info!("IP address: {}", config::IP_CIDR.address());
        info!(
            "MAC address: {}",
            EthernetAddress::from_bytes(&config::MAC_ADDRESS)
        );
        info!("Reset reason: {reset_reason}");
        info!("############################################################");

        let mut common_delay = ctx.device.TIM4.delay_ms(&clocks);

        let flash = ctx.device.FLASH;
        debug!("Flash addr: 0x{:X}", flash.address());
        debug!("Flash size: {} ({}K)", flash.len(), flash.len() / 1024);
        debug!("Flash dual-bank: {}", flash.dual_bank());
        for sector in flash_sectors(flash.len(), flash.dual_bank()) {
            debug!(
                "  sector {} @ 0x{:X}, LEN = {} ({}K)",
                sector.number,
                sector.offset,
                sector.size,
                sector.size / 1024
            );
        }

        info!("Setup: ETH");
        watchdog.feed();
        let eth_spi = {
            let sck = gpiob.pb13.into_alternate().speed(GpioSpeed::VeryHigh);
            let miso = gpiob.pb14.into_alternate().speed(GpioSpeed::VeryHigh);
            let mosi = gpiob
                .pb15
                .into_alternate()
                .speed(GpioSpeed::VeryHigh)
                .internal_pull_up(true);

            // TODO 3 MHz
            Spi::new(
                ctx.device.SPI2,
                (sck, miso, mosi),
                enc28j60::MODE,
                1.MHz(),
                &clocks,
            )
        };
        let mut eth = {
            let ncs = gpiob.pb12.into_push_pull_output_in_state(true.into());

            let mut int = gpioa.pa8.into_pull_up_input();
            int.make_interrupt_source(&mut syscfg);
            int.enable_interrupt(&mut ctx.device.EXTI);
            int.trigger_on_edge(&mut ctx.device.EXTI, Edge::Falling);

            let mut reset = gpiob.pb1.into_push_pull_output_in_state(true.into());

            // Perform a hard reset first, then let the driver
            // perform a soft reset by provided enc28j60::Unconnected
            // instead of the actual reset pin
            reset.set_low();
            common_delay.delay_ms(5_u8);
            reset.set_high();
            common_delay.delay_ms(5_u8);

            let mut enc = enc28j60::Enc28j60::new(
                eth_spi,
                ncs,
                int,
                enc28j60::Unconnected,
                &mut common_delay,
                7168,
                config::MAC_ADDRESS,
            )
            .unwrap();

            enc.listen(enc28j60::Event::Pkt).unwrap();

            Eth::new(
                enc,
                &mut ctx.local.eth_storage.rx_buffer[..],
                &mut ctx.local.eth_storage.tx_buffer[..],
            )
        };

        info!("Setup: TCP/IP");
        let mac = EthernetAddress::from_bytes(&config::MAC_ADDRESS);
        let mut config = Config::new();
        config.hardware_addr = Some(mac.into());
        let mut eth_iface = Interface::new(config, &mut eth);
        eth_iface.update_ip_addrs(|addr| {
            addr.push(config::IP_CIDR.into()).unwrap();
        });
        let mut sockets = SocketSet::new(&mut ctx.local.net_storage.sockets[..]);
        let udp_rx_buf = UdpPacketBuffer::new(
            &mut ctx.local.udp_socket_storage.rx_metadata[..],
            &mut ctx.local.udp_socket_storage.rx_buffer[..],
        );
        let udp_tx_buf = UdpPacketBuffer::new(
            &mut ctx.local.udp_socket_storage.tx_metadata[..],
            &mut ctx.local.udp_socket_storage.tx_buffer[..],
        );
        let udp_socket = UdpSocket::new(udp_rx_buf, udp_tx_buf);
        let udp_handle = sockets.add(udp_socket);

        info!("Setup: net clock timer");
        let mut net_clock_timer = ctx.core.SYST.counter_us(&clocks);
        net_clock_timer.start(1.millis()).unwrap();
        net_clock_timer.listen(SysEvent::Update);

        info!("Setup: net poll timer");
        let mut ipstack_poll_timer = ctx.device.TIM3.counter_hz(&clocks);
        ipstack_poll_timer.start(25.Hz()).unwrap();
        ipstack_poll_timer.listen(Event::Update);

        let mono = ctx.device.TIM2.monotonic_us(&clocks);
        info!(">>> Initialized <<<");
        watchdog.feed();

        watchdog_task::spawn().unwrap();

        // TODO - if GPIO is not pressed (debounced?)
        // then boot application
        //   maybe early on and skip RTIC setup entirely...
        //   or maybe a task, idk yet
        // else keep running, with timeout (5min?)

        while button.is_low() {
            cortex_m::asm::nop();
        }

        (
            Shared {
                eth,
                net: eth_iface,
                sockets,
                udp_socket: udp_handle,
            },
            Local {
                net_clock_timer,
                ipstack_poll_timer,
                led,
                watchdog,
                button,
            },
            init::Monotonics(mono),
        )
    }

    extern "Rust" {
        #[task(local = [watchdog, led])]
        fn watchdog_task(ctx: watchdog_task::Context);
    }

    extern "Rust" {
        #[task(binds = SysTick, local = [net_clock_timer])]
        fn ipstack_clock_timer_task(ctx: ipstack_clock_timer_task::Context);
    }

    extern "Rust" {
        #[task(shared = [eth, net, sockets], capacity = 2)]
        fn ipstack_poll_task(ctx: ipstack_poll_task::Context);
    }

    extern "Rust" {
        #[task(binds = TIM3, local = [ipstack_poll_timer])]
        fn ipstack_poll_timer_task(ctx: ipstack_poll_timer_task::Context);
    }

    extern "Rust" {
        #[task(binds = EXTI9_5, shared = [eth])]
        fn eth_gpio_interrupt_handler_task(ctx: eth_gpio_interrupt_handler_task::Context);
    }
}

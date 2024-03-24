use defmt::info;

use embassy_futures::join::join3;
use embassy_futures::select::{select, Either};
use embassy_usb::driver::EndpointError;

use crate::board::{Irqs, MidiUsb};
use crate::midi::{MidiChannelReceiver, MidiMsg};
use static_cell::StaticCell;

pub struct UsbMidi<'d> {
    usb: embassy_usb::UsbDevice<'d, embassy_rp::usb::Driver<'d, MidiUsb>>,
    midi_rx: MidiChannelReceiver<'d>,
    class_tx: embassy_usb::class::midi::Sender<'d, embassy_rp::usb::Driver<'d, MidiUsb>>,
    class_rx: embassy_usb::class::midi::Receiver<'d, embassy_rp::usb::Driver<'d, MidiUsb>>,
}

static DEVICE_DESCRIPTOR: StaticCell<[u8; 256]> = StaticCell::new();
static CONFIG_DESCRIPTOR: StaticCell<[u8; 256]> = StaticCell::new();
static BOS_DESCRIPTOR: StaticCell<[u8; 256]> = StaticCell::new();
static CONTROL_BUF: StaticCell<[u8; 256]> = StaticCell::new();

impl<'d> UsbMidi<'d> {
    pub fn new(usb_per: MidiUsb, midi_rx: MidiChannelReceiver<'d>) -> Self {
        let driver = embassy_rp::usb::Driver::new(usb_per, Irqs);

        // Create embassy-usb Config
        let mut config = embassy_usb::Config::new(0x6666, 0x4857);
        config.manufacturer = Some("Jajcus");
        config.product = Some("MIDI touch keyboard");
        config.serial_number = Some("00000001");
        config.max_power = 500;
        config.max_packet_size_0 = 64;

        // Required for windows compatibility.
        // https://developer.nordicsemi.com/nRF_Connect_SDK/doc/1.9.1/kconfig/CONFIG_CDC_ACM_IAD.html#help
        config.device_class = 0xEF;
        config.device_sub_class = 0x02;
        config.device_protocol = 0x01;
        config.composite_with_iads = true;

        let device_descriptor = DEVICE_DESCRIPTOR.init([0; 256]);
        let config_descriptor = CONFIG_DESCRIPTOR.init([0; 256]);
        let bos_descriptor = BOS_DESCRIPTOR.init([0; 256]);
        let control_buf = CONTROL_BUF.init([0; 256]);

        let mut builder = embassy_usb::Builder::new(
            driver,
            config,
            device_descriptor,
            config_descriptor,
            bos_descriptor,
            &mut [], // no msos descriptors
            control_buf,
        );

        let class = embassy_usb::class::midi::MidiClass::new(&mut builder, 1, 1, 64);
        let (class_tx, class_rx) = class.split();

        let usb = builder.build();

        Self {
            usb,
            midi_rx,
            class_tx,
            class_rx,
        }
    }
    pub async fn task(&mut self) -> ! {
        let usb_task = self.usb.run();

        let mut buf = [0; 64];

        let midi_send_task = async {
            loop {
                loop {
                    match select(self.class_tx.wait_connection(), self.midi_rx.receive()).await {
                        Either::First(_) => break,
                        Either::Second(_) => continue,
                    };
                }
                info!("Connected");

                let mut pos = 0;

                let add_msg = |buf: &mut [u8], pos: &mut usize, msg: MidiMsg| {
                    info!("usb: msg: {}", msg);

                    buf[*pos] = msg.usb_cin();
                    let num_bytes = msg.serialize(&mut buf[*pos + 1..*pos + 4]);
                    if num_bytes == 0 || num_bytes > 3 {
                        return;
                    }
                    if num_bytes < 3 {
                        buf[*pos + 3] = 0;
                    }
                    if num_bytes == 1 {
                        buf[*pos + 2] = 0;
                    }
                    *pos += 4;
                };

                loop {
                    if pos == 0 {
                        // USB buffer empty
                        add_msg(&mut buf, &mut pos, self.midi_rx.receive().await);
                        while pos < 60 {
                            if let Ok(msg) = self.midi_rx.try_receive() {
                                add_msg(&mut buf, &mut pos, msg);
                            } else {
                                break;
                            };
                        }
                    } else if pos >= 60 {
                        // USB buffer full
                        info!("usb: sending: {} (full)", &buf[0..pos]);
                        match self.class_tx.write_packet(&buf[0..pos]).await {
                            Ok(_) => {
                                info!("sent!");
                                pos = 0;
                            }
                            Err(EndpointError::BufferOverflow) => panic!("buffer overflow!"),
                            Err(EndpointError::Disabled) => break,
                        }
                    } else {
                        info!("usb: sending: {}", &buf[0..pos]);
                        match select(
                            self.midi_rx.receive(),
                            self.class_tx.write_packet(&buf[0..pos]),
                        )
                        .await
                        {
                            Either::First(msg) => {
                                add_msg(&mut buf, &mut pos, msg);
                                while pos < 60 {
                                    if let Ok(msg) = self.midi_rx.try_receive() {
                                        add_msg(&mut buf, &mut pos, msg);
                                    } else {
                                        break;
                                    };
                                }
                            }
                            Either::Second(Ok(_)) => {
                                info!("sent!");
                                pos = 0;
                            }
                            Either::Second(Err(EndpointError::BufferOverflow)) => {
                                panic!("buffer overflow!")
                            }
                            Either::Second(Err(EndpointError::Disabled)) => break,
                        }
                    };
                }
                info!("Disconnected");
            }
        };

        let midi_recv_task = async {
            let mut buf = [0; 64];
            loop {
                self.class_rx.wait_connection().await;
                info!("Connected (recv)");

                loop {
                    match self.class_rx.read_packet(&mut buf).await {
                        Ok(n) => {
                            info!("Received {} bytes\n", n);
                        }
                        Err(EndpointError::BufferOverflow) => panic!("buffer overflow!"),
                        Err(EndpointError::Disabled) => break,
                    }
                }
                info!("Disconnected (recv)");
            }
        };

        join3(usb_task, midi_send_task, midi_recv_task).await;

        unreachable!();
    }
}

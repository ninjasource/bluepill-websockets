#![no_std]
#![no_main]
#![allow(deprecated)]

extern crate panic_itm;

#[macro_use]
extern crate cortex_m;

use stm32f1xx_hal::{prelude::*};

use core::str::Utf8Error;
use cortex_m::{peripheral::itm::Stim};
use cortex_m_rt::entry;
use embedded_hal::spi::FullDuplex;
use embedded_websockets as ws;
use w5500::{IpAddress, MacAddress, Socket, SocketStatus, W5500};
use ws::{WebSocket, WebSocketReceiveMessageType, WebSocketSendMessageType, WebSocketState};

use embedded_hal::{spi::Mode, spi::Phase, spi::Polarity, digital::OutputPin};

use stm32f1xx_hal as hal;
use hal::spi::Spi;
use hal::stm32;

type SpiFullDuplex = FullDuplex<u8, Error = hal::spi::Error>;

#[derive(Debug)]
enum WebServerError {
    Io(hal::spi::Error),
    WebSocket(ws::Error),
    Utf8Error,
}

impl From<hal::spi::Error> for WebServerError {
    fn from(err: hal::spi::Error) -> WebServerError {
        WebServerError::Io(err)
    }
}

impl From<ws::Error> for WebServerError {
    fn from(err: ws::Error) -> WebServerError {
        WebServerError::WebSocket(err)
    }
}

impl From<Utf8Error> for WebServerError {
    fn from(_err: Utf8Error) -> WebServerError {
        WebServerError::Utf8Error
    }
}

struct Connection {
    pub web_socket: WebSocket,
    pub socket: Socket,
    pub socket_status: SocketStatus,
}

impl Connection {
    fn new(socket: Socket) -> Connection {
        Connection {
            web_socket: WebSocket::new_server(),
            socket,
            socket_status: SocketStatus::Closed,
        }
    }
}

#[entry]
fn main() -> ! {
    let mut cp: cortex_m::Peripherals = cortex_m::Peripherals::take().unwrap();
    let dp = stm32::Peripherals::take().unwrap();

    let itm = &mut cp.ITM;
    iprintln!(&mut itm.stim[0], "INFO Initializing TCP v1.19");

    let mut rcc = dp.RCC.constrain();
    let mut afio = dp.AFIO.constrain(&mut rcc.apb2);
    let mut flash = dp.FLASH.constrain();
    let mut gpioa = dp.GPIOA.split(&mut rcc.apb2);

    let button = gpioa.pa0.into_pull_up_input(&mut gpioa.crl);

    let clocks = rcc.cfgr.freeze(&mut flash.acr);
    let mut delay = hal::delay::Delay::new(cp.SYST, clocks);
    delay.delay_ms(250_u16);

    let sck = gpioa.pa5.into_alternate_push_pull(&mut gpioa.crl);
    let miso = gpioa.pa6;
    let mosi = gpioa.pa7.into_alternate_push_pull(&mut gpioa.crl);

    let mut cs_ethernet = gpioa.pa2.into_push_pull_output(&mut gpioa.crl);
    cs_ethernet.set_low(); // low is active, high is inactive

    let mut cs_sdcard = gpioa.pa1.into_push_pull_output(&mut gpioa.crl);
    cs_sdcard.set_low(); // low is active, high is inactive

    let mut spi = Spi::spi1(
        dp.SPI1,
        (sck, miso, mosi),
        &mut afio.mapr,
        Mode {
            polarity: Polarity::IdleLow,
            phase: Phase::CaptureOnFirstTransition,
        },
        1.mhz(), // upt to 8mhz for w5500 module, 2mhz is max for eeprom in 3.3V
        clocks,
        &mut rcc.apb2,
    );

    run_web_server(&mut spi, &mut itm.stim[0], &mut delay, &mut cs_ethernet, &button).unwrap();

    loop {

    }
}

fn run_web_server(spi: &mut SpiFullDuplex, itm: &mut Stim, delay : &mut hal::delay::Delay, cs_ethernet: &mut embedded_hal::digital::OutputPin, button: &embedded_hal::digital::InputPin) -> Result<(), WebServerError> {
    let root_html = "HTTP/1.1 200 OK\r\nContent-Type: text/html; charset=UTF-8\r\nContent-Length: 2591\r\nConnection: close\r\n\r\n<!doctype html>
<html>
<head>
    <meta content='text/html;charset=utf-8' http-equiv='Content-Type' />
    <meta content='utf-8' http-equiv='encoding' />
    <meta name='viewport' content='width=device-width, initial-scale=0.5, maximum-scale=0.5, user-scalable=0' />
    <meta name='apple-mobile-web-app-capable' content='yes' />
    <meta name='apple-mobile-web-app-status-bar-style' content='black' />
    <title>Web Socket Demo</title>
    <style type='text/css'>
        * { margin: 0; padding: 0; box-sizing: border-box; }
        body { font: 13px Helvetica, Arial; }
        form { background: #000; padding: 3px; position: fixed; bottom: 0; width: 100%; }
        form input { border: 0; padding: 10px; width: 90%; margin-right: .5%; }
        form button { width: 9%; background: rgb(130, 200, 255); border: none; padding: 10px; }
        #messages { list-style-type: none; margin: 0; padding: 0; }
        #messages li { padding: 5px 10px; }
        #messages li:nth-child(odd) { background: #eee; }
    </style>
</head>
<body>
    <ul id='messages'></ul>
    <form action=''>
    <input id='txtBox' autocomplete='off' /><button>Send</button>
    </form>
    <script type='text/javascript' src='http://code.jquery.com/jquery-1.11.1.js' ></script>
    <script type='text/javascript'>
        var CONNECTION;
        window.onload = function () {
            // open the connection to the Web Socket server
            // CONNECTION = new WebSocket('ws://' + location.host + ':80/chat');
			CONNECTION = new WebSocket('ws://192.168.1.33:1337/chat');

            // When the connection is open
            CONNECTION.onopen = function () {
                $('#messages').append($('<li>').text('Connection opened'));
            };

            // when the connection is closed by the server
            CONNECTION.onclose = function () {
                $('#messages').append($('<li>').text('Connection closed'));
            };

            // Log errors
            CONNECTION.onerror = function (e) {
                console.log('An error occured');
            };

            // Log messages from the server
            CONNECTION.onmessage = function (e) {
                $('#messages').append($('<li>').text(e.data));
            };
        };

		$(window).on('beforeunload', function(){
			CONNECTION.close();
		});

        // when we press the Send button, send the text to the server
        $('form').submit(function(){
            CONNECTION.send($('#txtBox').val());
            $('#txtBox').val('');
            return false;
        });
    </script>
</body>
</html>";

    let mut w5500 = W5500::new(cs_ethernet);

    w5500.set_mode(spi, false, false, false, false)?;
    w5500.set_mac(spi, &MacAddress::new(0x02, 0x01, 0x02, 0x03, 0x04, 0x05))?;
    w5500.set_ip(spi, &IpAddress::new(192, 168, 1, 33))?;
    w5500.set_subnet(spi, &IpAddress::new(255, 255, 255, 0))?;
    w5500.set_gateway(spi, &IpAddress::new(192, 168, 1, 1))?;

    const PORT: u16 = 1337;

    let mut buffer: [u8; 3000] = [0; 3000];
    let mut ws_buffer: [u8; 500] = [0; 500];

    const NUM_SOCKETS: usize = 8;

    let mut connections: [Connection; NUM_SOCKETS] = [
        Connection::new(Socket::Socket0),
        Connection::new(Socket::Socket1),
        Connection::new(Socket::Socket2),
        Connection::new(Socket::Socket3),
        Connection::new(Socket::Socket4),
        Connection::new(Socket::Socket5),
        Connection::new(Socket::Socket6),
        Connection::new(Socket::Socket7),
    ];

    // make sure all the connections are closed before we start
    for connection in connections.iter() {
        w5500.set_protocol(spi, connection.socket, w5500::Protocol::TCP)?;
        w5500.dissconnect(spi, connection.socket)?;
    }

    loop {
        for index in 0..NUM_SOCKETS {
            let mut connection = &mut connections[index];

            if button.is_low() {
                match w5500.read_registers(spi, connection.socket) {
                    Ok((_mode, command, status, interrupt, port, interrupt_mask)) => {
                        iprintln!(itm, "INFO Socket0 command: {:#X}", command);
                        iprintln!(itm, "INFO Socket0 status: {:#X}", status);
                        iprintln!(itm, "INFO Socket0 interrupt: {:b}", interrupt);
                        iprintln!(itm, "INFO Socket0 port: {}", port);
                        iprintln!(itm, "INFO Socket0 interrupt mask: {:b}", interrupt_mask);
                        iprintln!(itm);
                        delay.delay_ms(50_u16);
                    }
                    Err(_e) => iprintln!(itm, "INFO Read Registers Error"),
                }
            }

            match w5500.get_socket_status(spi, connection.socket) {
                Ok(Some(socket_status)) => {
                    if connection.socket_status != socket_status {
                        // print status change
                        iprintln!(
                            itm,
                            "INFO Socket status: {:?} -> {:?}",
                            connection.socket_status,
                            socket_status
                        );
                        if socket_status == SocketStatus::Closed {
                            iprintln!(itm);
                        }
                        connection.socket_status = socket_status;
                    }
                    match socket_status {
                        SocketStatus::Closed | SocketStatus::CloseWait => {
                            // open
                            iprintln!(itm, "INFO TCP Opening {:?}", connection.socket);
                            w5500.open_tcp(spi, connection.socket)?;
                        }
                        SocketStatus::Init => {
                            // listen
                            iprintln!(
                                itm,
                                "INFO TCP Attempting to listen to {:?} on port: {}",
                                connection.socket,
                                PORT
                            );
                            w5500.listen_tcp(spi, connection.socket, PORT)?;
                        }
                        SocketStatus::Established => {
                            eth_read(
                                spi,
                                connection.socket,
                                &mut w5500,
                                &mut connection.web_socket,
                                &mut buffer,
                                &mut ws_buffer,
                                &root_html,
                                itm,
                            )?;
                        }
                        _ => {} // do nothing
                    }
                },
                Ok(None) => {
                    iprintln!(itm, "ERROR Unknown socket status");
                    return Ok(())
                },
                Err(_e) => iprintln!(itm, "ERROR Cannot read socket status"),
            };
        }
    }
}

fn ws_write_back(
    spi: &mut SpiFullDuplex,
    socket: Socket,
    w5500: &mut W5500,
    web_socket: &mut WebSocket,
    eth_buffer: &mut [u8],
    ws_buffer: &mut [u8],
    count: usize,
    send_message_type: WebSocketSendMessageType,
    itm: &mut Stim,
) -> Result<(), WebServerError> {
    eth_buffer[..count].copy_from_slice(&ws_buffer[..count]);
    let ws_to_send = web_socket.write(send_message_type, true, &eth_buffer[..count], ws_buffer)?;
    eth_write(spi, socket, w5500, &ws_buffer[..ws_to_send], itm)?;
    iprintln!(
        itm,
        "INFO Websocket encoded {:#?}: {} bytes",
        send_message_type,
        ws_to_send
    );
    Ok(())
}

fn ws_read(
    spi: &mut SpiFullDuplex,
    socket: Socket,
    w5500: &mut W5500,
    web_socket: &mut WebSocket,
    eth_buffer: &mut [u8],
    ws_buffer: &mut [u8],
    itm: &mut Stim,
) -> core::result::Result<(), WebServerError> {
    let ws_read_result = web_socket.read(&eth_buffer, ws_buffer)?;
    iprintln!(
        itm,
        "INFO Websocket decoded {:#?}: {} bytes",
        ws_read_result.message_type,
        ws_read_result.num_bytes_to
    );
    match ws_read_result.message_type {
        WebSocketReceiveMessageType::Text => {
            {
                let message = ::core::str::from_utf8(&ws_buffer[..ws_read_result.num_bytes_to])?;
                iprintln!(itm, "INFO Websocket: {}", message);
            }

            ws_write_back(
                spi,
                socket,
                w5500,
                web_socket,
                eth_buffer,
                ws_buffer,
                ws_read_result.num_bytes_to,
                WebSocketSendMessageType::Text,
                itm,
            )?;
        }
        WebSocketReceiveMessageType::Binary => {
            // do nothing
        }
        WebSocketReceiveMessageType::CloseMustReply => {
            let close_status = ws_read_result.close_status.unwrap(); // this should never fail

            {
                if ws_read_result.num_bytes_to > 2 {
                    let message =
                        ::core::str::from_utf8(&ws_buffer[2..ws_read_result.num_bytes_to])?;
                    iprintln!(
                        itm,
                        "INFO Websocket close status {:#?}: {}",
                        close_status,
                        message
                    );
                } else {
                    iprintln!(itm, "INFO Websocket close status {:#?}", close_status);
                }
            }

            ws_write_back(
                spi,
                socket,
                w5500,
                web_socket,
                eth_buffer,
                ws_buffer,
                ws_read_result.num_bytes_to,
                WebSocketSendMessageType::CloseReply,
                itm,
            )?;
            w5500.close(spi, socket)?;
            iprintln!(itm, "INFO TCP connection closed");
        }
        WebSocketReceiveMessageType::Ping => {
            ws_write_back(
                spi,
                socket,
                w5500,
                web_socket,
                eth_buffer,
                ws_buffer,
                ws_read_result.num_bytes_to,
                WebSocketSendMessageType::Pong,
                itm,
            )?;
        }
        WebSocketReceiveMessageType::Pong => {
            // do nothing
        }
        WebSocketReceiveMessageType::CloseCompleted => {
            iprintln!(itm, "INFO Websocket close handshake completed");
            w5500.close(spi, socket)?;
            iprintln!(itm, "INFO TCP connection closed");
        }
    }

    Ok(())
}

fn eth_write(
    spi: &mut SpiFullDuplex,
    socket: Socket,
    w5500: &mut W5500,
    buffer: &[u8],
    itm: &mut Stim,
) -> Result<(), WebServerError> {
    let mut start = 0;
    loop {
        let bytes_sent = w5500.send_tcp(spi, socket, &buffer[start..])?;
        iprintln!(itm, "INFO Sent {} bytes", bytes_sent);
        start += bytes_sent;

        if start == buffer.len() {
            return Ok(());
        }
    }
}

fn send_html_and_close(
    spi: &mut SpiFullDuplex,
    socket: Socket,
    w5500: &mut W5500,
    //eth_buffer: &mut [u8],
    html: &str,
    itm: &mut Stim,
) -> Result<(), WebServerError> {
    iprintln!(itm, "INFO Sending: {}", html);
    eth_write(spi, socket, w5500, &html.as_bytes(), itm)?;
    w5500.close(spi, socket)?;
    iprintln!(itm, "INFO Send complete. Connection closed");
    Ok(())
}

fn eth_read(
    spi: &mut SpiFullDuplex,
    socket: Socket,
    w5500: &mut W5500,
    web_socket: &mut WebSocket,
    eth_buffer: &mut [u8],
    ws_buffer: &mut [u8],
    root_html: &str,
    itm: &mut Stim,
) -> Result<(), WebServerError> {
    let size = w5500.try_receive_tcp(spi, socket, eth_buffer)?;
    if let Some(size) = size {
        iprintln!(itm, "INFO Received {} bytes", size);
        if web_socket.get_state() == WebSocketState::Open {
            ws_read(spi, socket, w5500, web_socket, eth_buffer, ws_buffer, itm)?;
        } else {
            let http_header = ws::read_http_header(eth_buffer)?;
            if let Some(websocket_context) = http_header.websocket_context {
                iprintln!(itm, "INFO Websocket request. Generating handshake");
                let ws_send = web_socket.server_respond_to_opening_handshake(
                    &websocket_context.sec_websocket_key,
                    None,
                    eth_buffer,
                )?;
                iprintln!(
                    itm,
                    "INFO Websocket sending handshake response of {} bytes",
                    ws_send
                );
                w5500.send_tcp(spi, socket, &eth_buffer[..ws_send])?;
                iprintln!(itm, "INFO Websocket handshake complete");
            } else {
                iprintln!(itm, "INFO Http File header path: {}", http_header.path);
                match http_header.path.as_str() {
                    "/" => {
                        send_html_and_close(spi, socket, w5500, root_html, itm)?;
                    }
                    _ => {
                        let http = "HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\nConnection: close\r\n\r\n";
                        send_html_and_close(spi, socket, w5500, http, itm)?;
                    }
                }
            }
        }
    }

    Ok(())
}

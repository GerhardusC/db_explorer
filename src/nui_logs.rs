use std::{io::{Error, ErrorKind}, sync::mpsc::{self, Receiver, Sender}, thread, time::Duration};
use color_eyre::Result;

use cursive::{event::EventTrigger, view::Nameable, views::{Dialog, OnEventView, SelectView}, Cursive};
use mosquitto_rs::{Client, Event, QoS};
use tokio::sync::mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender};

struct ConnectOptions {}

enum LogEvent {
    Message(String),
    Disconnect,
}

enum ConnectionEvent {
    Connect(ConnectOptions),
    Disconnect,
    Reconnect(ConnectOptions),
}

pub fn draw_logs (s: &mut Cursive) {
    s.pop_layer();
    let (log_ui_event_sender, log_ui_event_receiver) = mpsc::channel::<LogEvent>();
    let (data_collection_event_sender, mut data_collection_event_receiver) = unbounded_channel::<ConnectionEvent>();
    let sender = log_ui_event_sender.clone();

    data_collection_event_sender.send(ConnectionEvent::Connect(ConnectOptions {  }));
    spawn_log_event_handler(s, data_collection_event_sender, log_ui_event_receiver);
    spawn_data_collection(s, data_collection_event_receiver, log_ui_event_sender);
    
    let view = OnEventView::new( Dialog::around(SelectView::<String>::new().with_name("nui_logs")))
        .on_event('s', move |s| {
            sender.send(LogEvent::Disconnect);
            s.add_layer(Dialog::info("Disconnected hopefully."));
        });
    // Draw UI elements
    s.add_layer(view);
}

fn spawn_log_event_handler (s: &mut Cursive, data_collection_event_sender: UnboundedSender<ConnectionEvent>, log_ui_event_receiver: Receiver<LogEvent>) {
    // UI event sender.
    let sink = s.cb_sink().to_owned();
    // We are forced to grab the receiver out of the async function, because we want to cancel
    // it from the other thread.
    // Spawn task that will call on UI to update UI
    thread::spawn(move || {
        // sink.send(Box::new(|s| {
        //
        // }));
        let sender = data_collection_event_sender.clone();
        sender.send(ConnectionEvent::Connect(ConnectOptions {  })); // Initial connect.

        while let Ok(event) = log_ui_event_receiver.recv() {
            let sender_cp = sender.clone();
            match event {
                LogEvent::Message(msg) => {
                    sink.send(Box::new(move |s| {
                        if let None = s.call_on_name("nui_logs", |v: &mut SelectView| {
                            v.add_item(msg.clone(), msg);
                        }) {
                            sender_cp.send(ConnectionEvent::Disconnect);
                        };
                    }));
                },
                LogEvent::Disconnect => {
                    sender_cp.send(ConnectionEvent::Disconnect);
                    // This return is very important.
                    return;
                },
            };
        }
    });
}


pub fn spawn_data_collection(s: &mut Cursive, mut data_collection_event_receiver: UnboundedReceiver<ConnectionEvent>, log_ui_event_sender: Sender<LogEvent>) {
    // let receiver_collection = receiver.clone();
    thread::spawn(move || {
        // Data collection thread that will pass back events to handler.
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();

        let res: Result<()> = rt.block_on(async {
            if let Some(evt) = data_collection_event_receiver.recv().await {
                match evt {
                    ConnectionEvent::Connect(connect_options) => {
                        let receiver = mqtt_create_client().await?;
                        receive_messages(&receiver.1, log_ui_event_sender).await?;
                    }
,
                    ConnectionEvent::Disconnect => {
                        data_collection_event_receiver.close();
                    },
                    ConnectionEvent::Reconnect(connect_options) => {
                        let receiver = mqtt_create_client().await?;
                        receive_messages(&receiver.1, log_ui_event_sender).await?;
                    },
                }

            };
            Ok(())
        });
    });
}

// TODO: Collect client opts.
async fn mqtt_create_client () -> Result<(Client, async_channel::Receiver<Event>)> {
    let client = Client::with_auto_id()?;
    let connection_status = client
        .connect("oldlaptop.local", 1883, Duration::from_secs(5), None).await?;

    client.subscribe("/#", QoS::AtMostOnce).await?;
    
    match client.subscriber() {
        Some(receiver) => return Ok((client, receiver)),
        None => return Err(Error::new(ErrorKind::Other, "You can only get one subscriber per client.").into()),
    }
}

async fn receive_messages(
    async_channel_receiver: &async_channel::Receiver<Event>,
    sender: Sender<LogEvent>,
) -> Result <()> {
    while let Ok(res) = async_channel_receiver.recv().await {
        match res {
            Event::Message(message) => {
                let msg_str = String::from_utf8(message.payload)
                    .unwrap_or_else(|e| format!("Failed to parse string: {}", e));

                let new_msg = format!("{}: {}", message.topic, msg_str);
                sender.send(LogEvent::Message(new_msg))?;
            }
            Event::Connected(connection_status) => {
                let new_msg = format!("MQTT Connected Event: {}", connection_status);
                sender.send(LogEvent::Message(new_msg))?;
            }
            Event::Disconnected(reason_code) => {
                let new_msg = format!("Disconnected: {}", reason_code);
                sender.send(LogEvent::Message(new_msg))?;
                return Err(Error::new(ErrorKind::Other, "Disconnected.").into());
            }
        }
    }
    Err(Error::new(ErrorKind::Other, "The data collection ended.").into())
}


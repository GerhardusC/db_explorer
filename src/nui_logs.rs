use std::{io::{Error, ErrorKind}, sync::{mpsc::{self, Sender}, Arc, Mutex}, thread, time::Duration};
use color_eyre::Result;

use async_channel::Receiver;
use cursive::{view::Nameable, views::{Dialog, SelectView}, Cursive};
use mosquitto_rs::{Client, Event, QoS};
use tokio::sync::mpsc::UnboundedSender;

struct ConnectOptions {}

enum LogEvent {
    Message(String),
    Disconnect,
}

enum ConnectionEvent {
    Connect(ConnectOptions),
}

pub fn draw_nui_logs(s: &mut Cursive) {
    s.pop_layer();

    // We can make the sender of events scoped here, so the sender can be passed across component.
    let (event_sender, event_receiver) = mpsc::channel::<LogEvent>();
    let (data_collection_event_sender, data_collection_event_receiver) = mpsc::channel::<ConnectionEvent>();

    let event_sender_mqtt = event_sender.clone();
    let event_sender_internal = event_sender.clone();
    
    // UI event sender.
    let sink = s.cb_sink().to_owned();
    // We are forced to grab the receiver out of the async function, because we want to cancel
    let receiver: Arc<Mutex<Option<Receiver<Event>>>> = Arc::new(Mutex::new(None));
    // it from the other thread.
    let receiver_cancel = receiver.clone();
    // Spawn task that will call on UI to update UI
    thread::spawn(move || {
        while let Ok(event) = event_receiver.recv() {
            let event_sender_internal_cp = event_sender_internal.clone();
            match event {
                LogEvent::Message(msg) => {
                    sink.send(Box::new(move |s| {
                        if let None = s.call_on_name("nui_logs", |v: &mut SelectView| {
                            v.add_item(msg.clone(), msg);
                        }) {
                            event_sender_internal_cp.send(LogEvent::Disconnect);
                        };
                    }));
                },
                LogEvent::Disconnect => {
                    let receiver = receiver_cancel.lock().unwrap();
                    if let Some(ref receiver) = *receiver {
                        receiver.close();
                    };
                },
            };
        }
    });
    // let receiver_collection = receiver.clone();
    thread::spawn(move || {
        // Data collection thread that will pass back events to handler.
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();

        rt.block_on(async {
            let new_receiver = mqtt_create_client().await.unwrap();
            if let Ok(mut receiver) = receiver.lock() {
                // The client itself is returned, but unused.
                *receiver = Some(new_receiver.1);
            }

            let event_sender_mqtt = event_sender_mqtt.clone();
            let active_receiver = receiver.lock().unwrap();
            if let Some(ref receiver) = *active_receiver {
                receive_messages(receiver, event_sender_mqtt).await;
            }

                // Give some time before locking the mux for a cancel signal to be read if
                // needed. Check if this can be removed later, maybe async runtime deals with
                // it.

            if let Ok(receiver) = receiver.lock() {
                // The client itself is returned, but unused.
                match *receiver {
                    Some(ref recv) => {
                        recv.close();
                    },
                    None => {},
                };
            }
        });

    });
    
    let view = Dialog::around(SelectView::<String>::new().with_name("nui_logs"));
    // Draw UI elements
    s.add_layer(view);

}

// TODO: Collect client opts.
async fn mqtt_create_client () -> Result<(Client, Receiver<Event>)> {
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
    async_channel_receiver: &Receiver<Event>,
    sender: Sender<LogEvent>,
){
    while let Ok(res) = async_channel_receiver.recv().await {
        match res {
            Event::Message(message) => {
                let msg_str = String::from_utf8(message.payload)
                    .unwrap_or_else(|e| format!("Failed to parse string: {}", e));

                let new_msg = format!("{}: {}", message.topic, msg_str);
                sender.send(LogEvent::Message(new_msg)).unwrap_or(());
            }
            Event::Connected(connection_status) => {
                let new_msg = format!("MQTT Connected Event: {}", connection_status);
                sender.send(LogEvent::Message(new_msg)).unwrap_or(());
            }
            Event::Disconnected(reason_code) => {
                let new_msg = format!("Disconnected: {}", reason_code);
                sender.send(LogEvent::Message(new_msg)).unwrap_or(());
            }
        }
    }
}


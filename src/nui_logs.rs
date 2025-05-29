use std::{io::{Error, ErrorKind}, sync::{mpsc::{self, Receiver, Sender}, Arc, Mutex}, thread, time::Duration};
use color_eyre::Result;

use cursive::{event::EventTrigger, view::Nameable, views::{Dialog, OnEventView, SelectView, TextView}, Cursive};
use mosquitto_rs::{Client, Event, QoS};
use tokio::{sync::mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender}, task::JoinHandle};

use crate::cli_args::ARGS;

#[derive(Clone)]
struct ConnectOptions {
    topic: String,
    host: String,
}

impl ConnectOptions {
    fn new(host: &str, topic: &str) -> Self {
        ConnectOptions { topic: topic.to_string(), host: host.to_string() }
    }
}

enum LogEvent {
    UpdateTopic(String),
    UpdateHost(String),
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
    let (data_collection_event_sender, data_collection_event_receiver) = unbounded_channel::<ConnectionEvent>();
    let sender = log_ui_event_sender.clone();
    let sender2 = log_ui_event_sender.clone();
    let sender3 = log_ui_event_sender.clone();

    data_collection_event_sender.send(ConnectionEvent::Connect(ConnectOptions::new(&ARGS.broker_ip , &ARGS.topic)));
    spawn_log_event_handler(s, data_collection_event_sender, log_ui_event_receiver);
    spawn_data_collection(s, data_collection_event_receiver, log_ui_event_sender);
    
    let view = OnEventView::new( Dialog::around(SelectView::<String>::new().with_name("nui_logs")))
        .on_event(EventTrigger::from_fn(|e| {
            match e {
                cursive::event::Event::FocusLost => true,
                cursive::event::Event::WindowResize => false,
                cursive::event::Event::Refresh => true,
                cursive::event::Event::Char(c) => {
                    match c {
                        &'q' => true,
                        &'t' => true,
                        &'l' => true,
                        _    => false,
                    }

                },
                cursive::event::Event::CtrlChar(_char) => false,
                cursive::event::Event::AltChar(_char) => false,
                cursive::event::Event::Key(_key) => false,
                cursive::event::Event::Shift(_key) => false,
                cursive::event::Event::Alt(_key) => false,
                cursive::event::Event::AltShift(_key) => false,
                cursive::event::Event::Ctrl(_key) => false,
                cursive::event::Event::CtrlShift(_key) => false,
                cursive::event::Event::CtrlAlt(_key) => false,
                // cursive::event::Event::Mouse { offset, position, event } =>false,
                cursive::event::Event::Unknown(_items) => false,
                _ => false,
            }
        }), move |s| {
                s.pop_layer();
                s.add_layer(Dialog::new().title("Disconnected hopefully.").content(TextView::new("Disconnected")));
                sender.send(LogEvent::Disconnect);
            }).on_event('s', move |s| {
                sender2.send(LogEvent::Disconnect);
            }).on_event('r', move |s| {
                // sender3.send(LogEvent::UpdateTopic()
                sender3.send(LogEvent::UpdateTopic("/#".to_owned()));
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
    // TODO: State here...
    thread::spawn(move || {
        let sender = data_collection_event_sender.clone();
        let state = Arc::new(Mutex::new(ConnectOptions::new(&ARGS.broker_ip, &ARGS.topic)));

        while let Ok(event) = log_ui_event_receiver.recv() {
            let state_cp = state.clone();
            let sender_cp = sender.clone();
            match event {
                LogEvent::UpdateTopic(res) => {
                    let mut old_topic = state_cp.lock().unwrap();
                    old_topic.topic = res.clone();
                    let mut new_topic = state_cp.lock().unwrap();
                    (*new_topic).topic = res.clone();
                    sender_cp.send(ConnectionEvent::Reconnect(ConnectOptions { topic: res.to_owned(), host: old_topic.host.clone() }));
                },
                LogEvent::UpdateHost(host) => {

                },
                LogEvent::Message(msg) => {
                    sink.send(Box::new(move |s| {
                        if let None = s.call_on_name("nui_logs", |v: &mut SelectView| {
                            v.add_item(msg.clone(), msg);
                        }) {
                            // Disconnect if logs view can't be found.
                            sender_cp.send(ConnectionEvent::Disconnect);
                            return;
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
    thread::spawn(move || {
        // Data collection thread that will pass back events to handler.
        let rt = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(4)
            .enable_all()
            .build()
            .unwrap();

        let res: Result<()> = rt.block_on(async {
            let client_mux: Arc<Mutex<Option<(Client, async_channel::Receiver<Event>)>>> = Arc::new(Mutex::new(None));
            while let Some(evt) = data_collection_event_receiver.recv().await {
                let log_ui_event_sender = log_ui_event_sender.clone();
                match evt {
                    ConnectionEvent::Connect(connect_options) => {
                        let receiver = mqtt_create_client(&connect_options.host, &connect_options.topic).await?;
                        {
                            let mut client = client_mux.lock().unwrap();
                            *client = Some(receiver.clone());
                        }
                        receive_messages(&receiver.1, log_ui_event_sender).await;
                    }
,
                    ConnectionEvent::Disconnect => {
                        let mut client = client_mux.lock().unwrap();
                        if (*client).is_some() {
                            
                            client.take().unwrap().1.close();
                            // drop(client.take().unwrap());
                        }
                        // return Ok(());
                    },
                    ConnectionEvent::Reconnect(connect_options) => {
                        let mut client = client_mux.lock().unwrap();
                        if (*client).is_some() {
                            client.take().unwrap().1.close();
                        }

                        let receiver = mqtt_create_client(&connect_options.host, &connect_options.topic).await?;
                        receive_messages(&receiver.1, log_ui_event_sender).await;
                    },
                }
            };
            Ok(())
        });
    });
}

// TODO: Collect client opts.
async fn mqtt_create_client (host: &str, topic: &str) -> Result<(Client, async_channel::Receiver<Event>)> {
    let client = Client::with_auto_id()?;
    let connection_status = client
        .connect(host, 1883, Duration::from_secs(5), None).await?;

    client.subscribe(topic, QoS::AtMostOnce).await?;
    
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


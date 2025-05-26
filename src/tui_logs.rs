use std::{
    sync::{Arc, Mutex},
    thread::{self},
    time::Duration,
};

use async_channel::Receiver;
use chrono::Utc;
use color_eyre::Result;
use cursive::{
    Cursive,
    view::Nameable,
    views::{Dialog, NamedView, OnEventView, ScrollView, SelectView},
};
use mosquitto_rs::{Client, Event, QoS};
use tokio::sync::mpsc::{self, UnboundedReceiver, UnboundedSender};

async fn receive_messages(
    async_channel_receiver: Arc<Mutex<Receiver<Event>>>,
    sender: UnboundedSender<String>,
) -> Result<()> {
    loop {
        if let Ok(res) = async_channel_receiver.lock() {
            let res = res.recv().await?;
            match res {
                Event::Message(message) => {
                    let msg_str = String::from_utf8(message.payload)
                        .unwrap_or_else(|e| format!("Failed to parse string: {}", e));

                    let new_msg = format!("{}: {}", message.topic, msg_str);
                    sender.send(new_msg)?;
                }
                Event::Connected(connection_status) => {
                    let new_msg = format!("MQTT Connected Event: {}", connection_status);
                    sender.send(new_msg)?;
                }
                Event::Disconnected(reason_code) => {
                    let new_msg = format!("Disconnected: {}", reason_code);
                    sender.send(new_msg)?;
                    return Ok(());
                }
            }
        }
    }
}

async fn log_collection_async(
    log_sender: UnboundedSender<String>,
    done_receiver: UnboundedReceiver<bool>,
) -> Result<()> {
    let client = Client::with_auto_id()?;
    if let Ok(e) = client
        .connect("oldlaptop.local", 1883, Duration::from_secs(5), None)
        .await
    {
        log_sender.send(format!("{}", e))?;
    };

    if let Err(e) = client.subscribe("/#", QoS::AtMostOnce).await {
        log_sender.send(format!("Err Subscribing: {}", e))?;
    };

    let subscriber = client.subscriber();
    if let Some(subscriber_receiver) = subscriber {
        race_done_receiver(log_sender, done_receiver, subscriber_receiver).await;
    } else {
        log_sender.send("No sub found...".to_owned())?;
    }
    Ok(())
}

async fn race_done_receiver(
    log_sender: UnboundedSender<String>,
    mut done_receiver: UnboundedReceiver<bool>,
    subscriber_receiver: Receiver<Event>,
) {
    let subscriber_receiver = Arc::new(Mutex::new(subscriber_receiver));
    let subscriber_receiver_cp = subscriber_receiver.clone();

    let sender_cp = log_sender.clone();
    let sender_cp_cp = log_sender.clone();

    tokio::select! {
        msg = done_receiver.recv() => {
            if let Some(_) = msg {
                let _ = sender_cp.send("Done".to_owned());
                if let Ok(subscriber_receiver_cp) = subscriber_receiver_cp.lock() {
                    subscriber_receiver_cp.close();
                }
            }
        }
        _ = receive_messages(subscriber_receiver, sender_cp_cp) => {
            let _ = log_sender.send("Got msg".to_owned());
        }
    }
}

fn spawn_data_collection_thread(
    log_sender: UnboundedSender<String>,
    done_receiver: UnboundedReceiver<bool>,
) {
    thread::spawn(|| {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build();

        if let Ok(rt) = rt {
            let _: Result<()> =
                rt.block_on(async move { log_collection_async(log_sender, done_receiver).await });
        }
    });
}

fn spawn_log_receiver_thread(s: &mut Cursive, mut log_receiver: UnboundedReceiver<String>) {
    let sink = s.cb_sink().clone();
    thread::spawn(move || {
        while let Some(msg) = log_receiver.blocking_recv() {
            let _ = sink.send(Box::new(move |s| {
                s.call_on_name("logs_view", |v: &mut SelectView| {
                    v.add_item(msg, Utc::now().to_rfc2822());
                });
            }));
        }
    });
}

pub fn draw_logs(s: &mut Cursive) {
    s.pop_layer();
    if let Some(_) = s.call_on_name("logs_view", |_v: &mut NamedView<Dialog>| {}) {
        s.pop_layer();
    };

    let (log_sender, log_receiver) = mpsc::unbounded_channel::<String>();
    let (done_sender, done_receiver) = mpsc::unbounded_channel::<bool>();

    spawn_data_collection_thread(log_sender, done_receiver);
    spawn_log_receiver_thread(s, log_receiver);

    s.add_layer(Dialog::around(ScrollView::new(
        OnEventView::new(SelectView::<String>::new().with_name("logs_view")).on_event(
            't',
            move |s| {
                s.add_layer(Dialog::info("Closed"));
                let _ = done_sender.send(true);
            },
        ),
    )))
}

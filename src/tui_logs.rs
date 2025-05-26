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
use tokio::sync::mpsc::{self, UnboundedSender};

async fn recv_msgs(
    subs: Arc<Mutex<Receiver<Event>>>,
    sender: UnboundedSender<String>,
) -> Result<()> {
    loop {
        let res = subs.lock().unwrap().recv().await?;
        match res {
            mosquitto_rs::Event::Message(message) => {
                let msg_str = String::from_utf8(message.payload)
                    .unwrap_or("Failed to parse string.".to_owned());

                let new_msg = format!("{}: {}", message.topic, msg_str);
                sender.send(new_msg)?;
            }
            mosquitto_rs::Event::Connected(connection_status) => {
                let new_msg = format!("MQTT Connected Event: {}", connection_status);
                sender.send(new_msg)?;
            }
            mosquitto_rs::Event::Disconnected(reason_code) => {
                let new_msg = format!("Disconnected: {}", reason_code);
                sender.send(new_msg)?;
                return Ok(());
            }
        }
    }
}

pub fn draw_logs(s: &mut Cursive) {
    s.pop_layer();
    if let Some(_) = s.call_on_name("logs_view", |_v: &mut NamedView<Dialog>| {}) {
        s.pop_layer();
    };

    let (sender, mut receiver) = mpsc::unbounded_channel::<String>();
    let (done_sender, mut done_receiver) = mpsc::unbounded_channel::<bool>();

    thread::spawn(|| {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build();

        if let Ok(rt) = rt {
            let _: Result<()> = rt.block_on(async move {
                let client = Client::with_auto_id()?;
                if let Ok(e) = client
                    .connect("oldlaptop.local", 1883, Duration::from_secs(5), None)
                    .await
                {
                    sender.send(format!("{}", e))?;
                };

                if let Err(e) = client.subscribe("/#", QoS::AtMostOnce).await {
                    sender.send(format!("Err Subscribing: {}", e))?;
                };

                let subscriber = client.subscriber();
                if let Some(subs) = subscriber {
                    let subs = Arc::new(Mutex::new(subs));
                    let subs_cp = subs.clone();

                    let sender1 = sender.clone();
                    let sender2 = sender.clone();

                    tokio::select! {
                        msg = done_receiver.recv() => {
                            if let Some(_) = msg {
                                let _ = sender1.send("Done".to_owned());
                                if let Ok(subs_cp) = subs_cp.lock() {
                                    subs_cp.close();
                                }
                            }
                        }
                        _ = recv_msgs(subs, sender) => {
                            let _ = sender2.send("Got msg".to_owned());
                        }
                    }
                } else {
                    sender.send("No sub found...".to_owned())?;
                }
                Ok(())
            });
        }
    });

    let sink = s.cb_sink().clone();
    thread::spawn(move || {
        while let Some(msg) = receiver.blocking_recv() {
            let _ = sink.send(Box::new(move |s| {
                s.call_on_name("logs_view", |v: &mut SelectView| {
                    v.add_item(msg, Utc::now().to_rfc2822());
                });
            }));
        }
    });

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

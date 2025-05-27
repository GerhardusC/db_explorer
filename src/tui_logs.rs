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
    theme::{BaseColor, Color, ColorStyle, Effect, Style},
    view::Nameable,
    views::{
        Button, Dialog, EditView, LinearLayout, NamedView, OnEventView, ScrollView, SelectView,
        TextView,
    },
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
    mut topic_receiver: UnboundedReceiver<String>,
) -> Result<()> {
    // let topic = topic_receiver.recv().await.unwrap_or_else(|| {"/#".to_owned()});
    let done_receiver = Arc::new(Mutex::new(done_receiver));

    while let Some(topic) = topic_receiver.recv().await {
        let client = Client::with_auto_id()?;
        if let Ok(e) = client
            .connect("oldlaptop.local", 1883, Duration::from_secs(5), None)
            .await
        {
            log_sender.send(format!("{}", e))?;
        };

        let done_receiver_cp = done_receiver.clone();
        let log_sender = log_sender.clone();
        if let Err(e) = client.subscribe(&topic, QoS::AtMostOnce).await {
            log_sender.send(format!("Err Subscribing: {}", e))?;
        };

        let subscriber = client.subscriber();
        if let Some(subscriber_receiver) = subscriber {
            race_done_receiver(log_sender, done_receiver_cp, subscriber_receiver).await;
        } else {
            log_sender.send("No sub found...".to_owned())?;
        }
    }

    Ok(())
}

async fn race_done_receiver(
    log_sender: UnboundedSender<String>,
    done_receiver: Arc<Mutex<UnboundedReceiver<bool>>>,
    subscriber_receiver: Receiver<Event>,
) {
    let subscriber_receiver = Arc::new(Mutex::new(subscriber_receiver));
    let subscriber_receiver_cp = subscriber_receiver.clone();

    let sender_cp = log_sender.clone();
    let sender_cp_cp = log_sender.clone();

    if let Ok(mut done_receiver) = done_receiver.lock() {
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
    } else {
        let _ = log_sender.send("Failed to lock mutex on done race.".to_owned());
    }
}

fn spawn_data_collection_thread(
    log_sender: UnboundedSender<String>,
    done_receiver: UnboundedReceiver<bool>,
    topic_receiver: UnboundedReceiver<String>,
) {
    thread::spawn(|| {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build();

        if let Ok(rt) = rt {
            let _: Result<()> = rt.block_on(async move {
                log_collection_async(log_sender, done_receiver, topic_receiver).await
            });
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
    if let Some(_) = s.call_on_name("logs_view", |_v: &mut NamedView<SelectView>| {}) {
        s.pop_layer();
    };

    let (log_sender, log_receiver) = mpsc::unbounded_channel::<String>();
    let (done_sender, done_receiver) = mpsc::unbounded_channel::<bool>();
    let (topic_sender, topic_receiver) = mpsc::unbounded_channel::<String>();

    spawn_data_collection_thread(log_sender, done_receiver, topic_receiver);
    spawn_log_receiver_thread(s, log_receiver);

    let done_sender_cp = done_sender.clone();
    let topic_sender_cp = topic_sender.clone();
    let buttons = LinearLayout::horizontal()
        .child(Button::new("EDIT TOPIC", move |s| {
            let topic_sender_cp = topic_sender.clone();
            let topic_sender_cp_cp = topic_sender.clone();
            let done_sender_cp = done_sender_cp.clone();
            let done_sender_cp_cp = done_sender_cp.clone();

            s.add_layer(
                Dialog::around(
                    EditView::new()
                        .on_edit(|s, val, size| {
                            s.call_on_name("current_topic", |v: &mut TextView| {
                                v.set_content(val);
                            });
                        })
                        .on_submit(move |s, val| {
                            s.call_on_name("current_topic", |v: &mut TextView| {
                                done_sender_cp.send(true);
                                let val = v.get_content().source().to_owned();
                                topic_sender_cp.send(val);
                            });
                            s.pop_layer();
                        }),
                )
                .title("New topic")
                .button("Ok", move |s| {
                    s.call_on_name("current_topic", |v: &mut TextView| {
                        done_sender_cp_cp.send(true);
                        let val = v.get_content().source().to_owned();
                        topic_sender_cp_cp.send(val);
                    });
                    s.pop_layer();
                }),
            );
        }))
        .child(Button::new("CLEAR LOG", |s| {
            s.call_on_name("logs_view", |v: &mut SelectView| {
                v.clear();
            });
        }))
        .child(
            TextView::new("Current topic: ")
                .style(Style::from(Effect::Bold))
                .style(Style::from(ColorStyle::new(
                    Color::Dark(BaseColor::Black),
                    Color::Dark(BaseColor::White),
                ))),
        )
        .child(TextView::new("/#").with_name("current_topic"));

    topic_sender_cp.send("/#".to_owned());
    let logs_view = Dialog::around(ScrollView::new(
        OnEventView::new(SelectView::<String>::new().with_name("logs_view")).on_event(
            't',
            move |s| {
                s.add_layer(Dialog::info("Closed"));
                let _ = done_sender.send(true);
            },
        ),
    ));

    let container = LinearLayout::vertical().child(buttons).child(logs_view);

    s.add_layer(container)
}

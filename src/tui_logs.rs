use std::{
    sync::{Arc, Mutex},
    thread::{self},
    time::Duration,
};

use async_channel::Receiver;
use chrono::{Local, Utc};
use color_eyre::{owo_colors::OwoColorize, Result};
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

use crate::cli_args::ARGS;

async fn receive_messages(
    async_channel_receiver: Receiver<Event>,
    sender: UnboundedSender<String>,
) -> Result<()> {
    loop {
        let res = async_channel_receiver.recv().await?;
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

enum UIEvent {
    UpdateTopic(String),
    UpdateHost(String),
}

struct UIState {
    topic: String,
    host: String,
}

async fn log_collection_async(
    log_sender: UnboundedSender<String>,
    done_receiver: UnboundedReceiver<bool>,
    mut ui_event_receiver: UnboundedReceiver<UIEvent>,
) -> Result<()> {
    let done_receiver = Arc::new(Mutex::new(done_receiver));
    // UI state can live here.
    let state = Arc::new(Mutex::new(UIState{
        topic: (&ARGS.topic).to_owned(),
        host: (&ARGS.broker_ip).to_owned()
    }));

    while let Some(ui_event) = ui_event_receiver.recv().await {
        let state_cp = state.clone();
        let client = Client::with_auto_id()?;
        
        match ui_event {
            UIEvent::UpdateTopic(new_topic) => {
                if let Ok(mut state) = state_cp.lock() {
                    (*state).topic = new_topic;
                };
            },
            UIEvent::UpdateHost(new_host) => {
                if let Ok(mut state) = state_cp.lock() {
                    (*state).host = new_host;
                };
            },
        }

        let host;
        let topic;
        // Creating a scope here and reading the host of state to avoid locking up the
        // state for too long.
        {
            let lock = state_cp.lock();
            match lock {
                Ok(ref state) => {
                    host = (*state).host.to_owned();
                    topic = (*state).topic.to_owned();
                },
                Err(_) => {
                    host = "localhost".to_owned();
                    topic = "/#".to_owned();
                },
            }
        }

        if let Ok(e) = client
            .connect(&host, 1883, Duration::from_secs(5), None)
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
    let subscriber_receiver = subscriber_receiver.clone();
    let subscriber_receiver_cp = subscriber_receiver.clone();

    let sender_cp = log_sender.clone();
    let sender_cp_cp = log_sender.clone();

    if let Ok(mut done_receiver) = done_receiver.lock() {
        tokio::select! {
            msg = done_receiver.recv() => {
                if let Some(_) = msg {
                    let _ = sender_cp.send("Done".to_owned());
                    subscriber_receiver_cp.close();
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
    ui_event_receiver: UnboundedReceiver<UIEvent>,
) {
    thread::spawn(|| {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build();

        if let Ok(rt) = rt {
            let _: Result<()> = rt.block_on(async move {
                log_collection_async(log_sender, done_receiver, ui_event_receiver).await
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
                    // Really expensive, but I like the items coming in at the top, because the
                    // newest is always visible then.
                    v.insert_item(
                        0,
                        Local::now().naive_local().format("%Y/%m/%d %H:%M:%S").to_string() + "-> " + &msg,
                        Utc::now().to_rfc2822()
                    );
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
    let (topic_sender, topic_receiver) = mpsc::unbounded_channel::<UIEvent>();

    // This thread is responsible for connecting and reconnecting to the mosquitto instance.
    spawn_data_collection_thread(log_sender, done_receiver, topic_receiver);

    // This thread is responsible for receiving logs from mosquitto, and 
    spawn_log_receiver_thread(s, log_receiver);

    let done_sender_cp = done_sender.clone();
    let done_sender_cp_cp = done_sender.clone();
    let topic_sender_cp = topic_sender.clone();
    let topic_sender_cp_cp = topic_sender.clone();

    let buttons = LinearLayout::vertical()
        .child(Button::new("EDIT HOST", move |s| {
            let event_sender1 = topic_sender_cp.clone();
            let done_sender1 = done_sender_cp.clone();
            let view = EditFieldDialogCreator::new(
                event_sender1, done_sender1, FieldToUpdate::Host
            );
            s.add_layer(view.create_view());

        }))
        .child(Button::new("EDIT TOPIC", move |s| {
            let event_sender1 = topic_sender_cp_cp.clone();
            let done_sender1 = done_sender_cp_cp.clone();
            let view = EditFieldDialogCreator::new(
                event_sender1, done_sender1, FieldToUpdate::Topic
            );
            s.add_layer(view.create_view());

        }))
        .child(Button::new("CLEAR LOG", |s| {
            s.call_on_name("logs_view", |v: &mut SelectView| {
                v.clear();
            });
        }));

    let labels = LinearLayout::vertical()
        .child(LinearLayout::horizontal()
            .child(
                TextView::new("Current host:  ")
                    .style(Style::from(Effect::Bold))
                    .style(Style::from(ColorStyle::new(
                        Color::Dark(BaseColor::Black),
                        Color::Dark(BaseColor::White),
                    ))),
            )
            .child(TextView::new(&ARGS.broker_ip).with_name("current_host"))
        )
        .child(LinearLayout::horizontal()
            .child(
                TextView::new("Current topic: ")
                    .style(Style::from(Effect::Bold))
                    .style(Style::from(ColorStyle::new(
                        Color::Dark(BaseColor::Black),
                        Color::Dark(BaseColor::White),
                    ))),
            )
            .child(TextView::new(&ARGS.topic).with_name("current_topic"))
        );

    let form = LinearLayout::horizontal()
        .child(buttons)
        .child(labels);

    // Initial message. Both topic sender and done sender are consumed by this step.
    topic_sender.send(UIEvent::UpdateTopic((&ARGS.topic).to_owned()));
    let logs_view = Dialog::around(ScrollView::new(
        OnEventView::new(SelectView::<String>::new().with_name("logs_view")).on_event(
            't',
            move |s| {
                s.add_layer(Dialog::info("Closed"));
                let _ = done_sender.send(true);
            },
        ),
    ));

    let container = LinearLayout::vertical().child(form).child(logs_view);

    s.add_layer(container)
}


#[derive(Clone)]
enum FieldToUpdate {
    Topic,
    Host,
}

impl FieldToUpdate {
    fn get_element_name(&self) -> &str {
        match self {
            FieldToUpdate::Topic => "current_topic",
            FieldToUpdate::Host => "current_host",
        }
    }

    fn into_ui_event(&self, val: String) -> UIEvent {
        match self {
            FieldToUpdate::Topic => UIEvent::UpdateTopic(val),
            FieldToUpdate::Host => UIEvent::UpdateHost(val),
        }
    }
}

#[derive(Clone)]
struct EditFieldDialogCreator {
    event_sender: UnboundedSender<UIEvent>,
    done_sender: UnboundedSender<bool>,
    field_to_update: FieldToUpdate,
}

impl EditFieldDialogCreator {
    fn new(
        event_sender: UnboundedSender<UIEvent>,
        done_sender: UnboundedSender<bool>,
        field_to_update: FieldToUpdate
    ) -> EditFieldDialogCreator {
        EditFieldDialogCreator{ event_sender, done_sender, field_to_update }
    }

    /** Consumes self to create view.*/
    fn create_view(self) -> Dialog {
        // We need a clone of sender for each submit and button.
        let self_clone = self.clone();
        Dialog::around(
            EditView::new()
                .on_submit(move |s, val| {
                    // Use 1
                    self_clone.handle_update(s, Some(val));
                }),
        )
        .title("New topic")
        .button("Ok", move |s| {
            // Use 2
            self.handle_update(s, None);
        })
        .button("Cancel", |s| {
            s.pop_layer();
        })
    }

    fn handle_update(&self, s: &mut Cursive, val: Option<&str>) {
        s.call_on_name(self.field_to_update.get_element_name(), |v: &mut TextView| {
            if let Some(val) = val {
                v.set_content(val);
            }
            self.done_sender.send(true);
            let val = v.get_content().source().to_owned();
            self.event_sender.send(self.field_to_update.into_ui_event(val));
        });
        s.pop_layer();
    }
}


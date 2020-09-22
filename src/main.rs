//! A uno game for now
//! Might make a protocol that allows other games
//! But for now lets get this working

mod config;
mod network;

use async_std::{io, task};
use cursive::align::HAlign;
use cursive::traits::*;
use cursive::Cursive;
use env_logger::{Builder, Env};
use futures::prelude::*;
use libp2p::gossipsub::protocol::MessageId;
use libp2p::gossipsub::{GossipsubEvent, GossipsubMessage, MessageAuthenticity, Topic};
use libp2p::{gossipsub, identity, PeerId};
use serde::{Deserialize, Serialize};
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::thread;
use std::time::Duration;
use std::{
    error::Error,
    task::{Context, Poll},
};

use libp2p::{
    floodsub::{self, Floodsub, FloodsubEvent},
    mdns::{Mdns, MdnsEvent},
    ping::{Ping, PingConfig},
    swarm::NetworkBehaviourEventProcess,
    Multiaddr, Swarm,
};

use cursive::view::ScrollStrategy;
use cursive::views::{
    BoxView, Button, Dialog, DummyView, EditView, LinearLayout, ResizedView, ScrollView,
    SelectView, TextView,
};

use cursive::traits::*;
use std::sync::{Arc, Mutex};

#[derive(Deserialize)]
struct Response {
    t: Time,
    m: Vec<MessageResp>,
}

#[derive(Deserialize)]
struct MessageResp {
    d: Message,
}

#[derive(Deserialize)]
struct Time {
    t: String,
}

#[derive(Serialize, Deserialize)]
struct Message {
    uuid: String,
    text: String,
}

fn main() -> Result<(), Box<dyn Error>> {
    // chat();
    gui()
}

fn gui() -> Result<(), Box<dyn Error>> {
    // Builder::from_env(Env::default().default_filter_or("info")).init();
    let mut rt = tokio::runtime::Runtime::new().unwrap();

    // Create two channels, one for the channel name
    // one to send new messages to the sub function

    let (net_sender, net_reciever): (Sender<Message>, Receiver<Message>) = channel();
    let (msg_sender, msg_reciever): (Sender<Message>, Receiver<Message>) = channel();

    // Create a random PeerId
    let local_key = identity::Keypair::generate_ed25519(); // Generate key
    let local_peer_id = PeerId::from(local_key.public()); // Generate the peer id using key
                                                          // println!("Local peer id: {:?}", local_peer_id);

    // Set up an encrypted TCP Transport over the Mplex and Yamux protocols
    let transport = libp2p::build_development_transport(local_key.clone())?;

    // Create a Gossipsub topic
    let topic = Topic::new("Global".into());

    // Create a Swarm to manage peers and events
    let mut swarm = {
        // to set default parameters for gossipsub use:
        // let gossipsub_config = gossipsub::GossipsubConfig::default();

        // To content-address message, we can take the hash of message and use it as an ID.
        let message_id_fn = |message: &GossipsubMessage| {
            let mut s = DefaultHasher::new();
            message.data.hash(&mut s);
            MessageId::from(s.finish().to_string())
        };

        // set custom gossibsub
        let gossibsub_config = gossipsub::GossipsubConfigBuilder::new()
            .heartbeat_interval(Duration::from_secs(10))
            .message_id_fn(message_id_fn) // content-address message. No two messages
            // of the same content will be propogated
            .build();
        // build a gossipsub network behavior
        let mut gossipsub =
            gossipsub::Gossipsub::new(MessageAuthenticity::Signed(local_key), gossibsub_config);
        gossipsub.subscribe(topic.clone());
        libp2p::Swarm::new(transport, gossipsub, local_peer_id)
    };

    // Listen on all interfaces and whatever port the OS assigns
    libp2p::Swarm::listen_on(&mut swarm, "/ip4/0.0.0.0/tcp/0".parse().unwrap()).unwrap();

    let cfg = config::prompt();

    // Reach out to another node if specified
    if let Some(to_dial) = cfg.get_network() {
        let dialing = to_dial.clone();
        match to_dial.parse() {
            Ok(to_dial) => match libp2p::Swarm::dial_addr(&mut swarm, to_dial) {
                Ok(_) =>
                    /* println!("Dialed {:?}", dialing)*/
                    {}
                Err(e) =>
                    /*println!("Dial {:?} failed: {:?}", dialing, e)*/
                    {}
            },
            Err(err) =>
                /*println!("Failed to parse address to dial: {:?}", err)*/
                {}
        }
    }

    let sub_swarm = Arc::new(Mutex::new(swarm));

    let pub_swarm = Arc::clone(&sub_swarm);

    let addr_swarm = Arc::clone(&sub_swarm);

    let msg_sender_new = msg_sender.clone();
    std::thread::spawn(move || loop {
        if let Ok(m) = net_reciever.recv() {
            if let Err(e) = pub_swarm
                .lock()
                .unwrap()
                .publish(&topic, serde_json::to_string(&m).unwrap().as_bytes())
            {
            } else {
                println!("pub");
                msg_sender_new.send(m).unwrap();
            }
        }
    });

    std::thread::spawn(move || {
        let mut listening = false;
        // Kick it off
        let _handle: Result<(), Box<dyn Error>> =
            task::block_on(future::poll_fn(move |cx: &mut Context<'_>| {
                // Get message
                let mut swarm = sub_swarm.lock().unwrap();

                loop {
                    match swarm.poll_next_unpin(cx) {
                        Poll::Ready(Some(gossip_event)) => match gossip_event {
                            GossipsubEvent::Message(peer_id, id, message) => {
                                msg_sender.send(serde_json::from_str(
                                    &String::from_utf8_lossy(&message.data).to_string()[..],
                                )?)?;
                            }
                            _ => {}
                        },
                        Poll::Ready(None) | Poll::Pending => {
                            println!("Broke");
                            break;
                        }
                    }
                }

                if !listening {
                    for m in libp2p::Swarm::listeners(&swarm) {
                        msg_sender.send(Message {
                            uuid: "Listening on".to_string(),
                            text: m.to_string(),
                        })?;
                    }
                    listening = true;
                }

                // match swarm.next_event().await {}
                // let e = rt.block_on(swarm.next_event());
                // println!("{:?}", e);

                Poll::Pending
            }));
        println!("It reaches here: subber");
    });

    let mut siv = cursive::default();
    siv.add_layer(ResizedView::with_fixed_size(
        (40, 20),
        Dialog::new()
            .title("Chat")
            .content(
                LinearLayout::vertical()
                    .child(
                        ScrollView::new(
                            LinearLayout::vertical()
                                .child(DummyView.fixed_height(1))
                                .with(|messages| {
                                    for _ in 0..13 {
                                        messages.add_child(DummyView.fixed_height(1));
                                    }
                                })
                                .child(DummyView.fixed_height(1))
                                .with_name("messages"),
                        )
                        .scroll_strategy(ScrollStrategy::StickToBottom),
                    )
                    .child(EditView::new().with_name("message")),
            )
            .h_align(HAlign::Center)
            .button("Send", move |s| {
                let message = s
                    .call_on_name("message", |view: &mut EditView| view.get_content())
                    .unwrap();
                if message.is_empty() {
                    s.add_layer(
                        Dialog::new()
                            .title("Chat")
                            .content(TextView::new("Please enter a message!"))
                            .button("Okay", |s| {
                                s.pop_layer();
                            }),
                    )
                } else {
                    if let Err(e) = net_sender.send(Message {
                        uuid: cfg.get_user().clone(),
                        text: (*message).clone(),
                    }) {
                        s.add_layer(
                            Dialog::new()
                                .title("Chat")
                                .content(TextView::new("Error Publishing!"))
                                .button("Okay", |s| {
                                    s.pop_layer();
                                }),
                        )
                    } else {
                        s.call_on_name("message", |view: &mut EditView| view.set_content(""))
                            .unwrap();
                    }
                }
            })
            .button("Quit", |s| s.quit()),
    ));

    let mut message_count = 0;
    siv.refresh();
    loop {
        siv.step();
        if !siv.is_running() {
            break;
        }

        let mut needs_refresh = false;
        // Non blocking channel reciever
        for m in msg_reciever.try_iter() {
            siv.call_on_name("messages", |messages: &mut LinearLayout| {
                needs_refresh = true;
                message_count += 1;
                messages.add_child(TextView::new(format!("{}: {}", m.uuid, m.text)));
                if message_count <= 14 {
                    messages.remove_child(0);
                }
            });
        }
        if needs_refresh {
            siv.refresh();
        }
    }

    Ok(())
}

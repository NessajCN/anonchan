use std::{collections::HashSet, str::FromStr, time::Duration};

use serde_json::{json, Value};
use socketioxide::{
    adapter::Adapter,
    extract::{AckSender, Data, SocketRef, State},
    socket::{DisconnectReason, Sid},
};
use tracing::{error, info, warn};

use serde::{Deserialize, Serialize};

use super::state::{OnlineDevs, OnlineUsers};

#[derive(Deserialize, Serialize, Debug, Clone)]
#[serde(transparent)]
struct Username(String);

#[derive(Deserialize, Serialize, Debug, Clone)]
struct Topic {
    title: String,
    tid: Sid,
}

#[derive(Serialize, Debug)]
struct AckReply<T: Serialize> {
    success: bool,
    message: T,
}

pub async fn on_disconnect<A: Adapter>(
    s: SocketRef<A>,
    reason: DisconnectReason,
    onlinedevs: State<OnlineDevs>,
    onlineusers: State<OnlineUsers>,
) {
    info!("{} has disconnected. Reason: {:?}", &s.id, reason);
    // sending to all clients in the room (channel) except sender
    if let Some(topic) = s.extensions.get::<Topic>() {
        s.to(topic.title).emit("hangup", &s.id).await.ok();
    }
    if let Some(user) = onlineusers.get(&s.id).await {
        onlineusers.remove(&s.id).await;

        let u = onlineusers.val().await;
        for d in u {
            let s = s.clone();
            tokio::spawn(async move {
                s.to(d).emit("userOffline", &s.id).await.ok();
            });
            // s.to(d.to_owned()).emit("userOffline", &s.id).await.ok();
        }
        info!("disconnected user:{user}");
    }
    if let Some(dev) = onlinedevs.get(&s.id).await {
        onlinedevs.remove(&s.id).await;
        onlinedevs.speaker_off(&dev).await;
        info!("disconnected device:{dev}");
    }
}

pub async fn on_identify<A: Adapter>(
    s: SocketRef<A>,
    Data(user): Data<String>,
    onlineusers: State<OnlineUsers>,
) {
    s.join(user.to_owned());
    onlineusers.add(s.id.to_owned(), user.to_owned()).await;
    info!("logged in: {}", &user);

    // send back online users list to sender.
    let e = onlineusers.entries().await;
    s.emit("refreshUsers", &[e]).ok();

    // notify all other users about this new online user.
    let msg = [(s.id.to_string(), user)];

    let u = onlineusers.val().await;
    for d in u {
        let s = s.clone();
        let msg = msg.clone();
        tokio::spawn(async move {
            if let Err(err) = s.to(d.clone()).emit("userOnline", &msg).await {
                error!("Error on identify handler when notifying {d}: {err}");
            }
        });
    }
    // for d in onlineusers.val().await.iter() {
    //     if let Err(err) = s.to(d.to_owned()).emit("userOnline", &msg).await {
    //         error!("Error on identify handler when notifying {d}: {err}");
    //     }
    // }
}

pub async fn on_signout<A: Adapter>(s: SocketRef<A>, onlineusers: State<OnlineUsers>) {
    let msgout = json!(s.id.to_string());
    if let Some(user) = onlineusers.get(&s.id).await {
        info!("{} has signed out", &user);
    }

    let u = onlineusers.val().await;
    for d in u {
        let s = s.clone();
        let msgout = msgout.clone();
        tokio::spawn(async move {
            s.to(d).emit("userOffline", &msgout).await.ok();
        });
        // s.to(d.to_owned()).emit("userOffline", &s.id).await.ok();
    }
    // for d in onlineusers.val().await.iter() {
    //     s.to(d.to_owned()).emit("userOffline", &msgout).await.ok();
    // }
    onlineusers.remove(&s.id).await;
}

pub async fn on_fetchuser<A: Adapter>(s: SocketRef<A>, onlineusers: State<OnlineUsers>) {
    let e = onlineusers.entries().await;
    s.emit("refreshUsers", &[e]).ok();
}

pub async fn on_message<A: Adapter>(s: SocketRef<A>, Data(msg): Data<Value>, ack: AckSender) {
    if let Some(devroom) = s.extensions.get::<Topic>() {
        match s
            .to(devroom.title)
            .timeout(Duration::from_secs(5))
            .emit_with_ack::<Value, Value>("message", &msg)
            .await
        {
            Ok(ack_stream) => {
                if let Ok(ack_s) = ack_stream.await {
                    ack.send(&ack_s).ok();
                } else {
                    ack.send(&AckReply {
                        success: false,
                        message: "Failed to send message",
                    })
                    .ok();
                }
            }
            Err(_) => {
                ack.send(&AckReply {
                    success: false,
                    message: "Failed to send message",
                })
                .ok();
            }
        }
    }
}

pub async fn on_checkbox<A: Adapter>(
    _s: SocketRef<A>,
    Data(devs): Data<Vec<String>>,
    ack: AckSender,
    onlinedevs: State<OnlineDevs>,
) {
    let mut devset = onlinedevs.val().await;
    devset.retain(|d| devs.contains(d) || d.starts_with("Unbound"));
    let mut devvec: Vec<String> = devset.into_iter().collect();
    devvec.sort();
    ack.send(&AckReply {
        success: true,
        message: devvec,
    })
    .ok();
}

pub async fn on_boxconf<A: Adapter>(
    s: SocketRef<A>,
    Data(devid): Data<String>,
    ack: AckSender,
    onlinedevs: State<OnlineDevs>,
) {
    let sid = match onlinedevs.getcamid(&devid).await {
        Some(s) => s,
        None => {
            ack.send(&AckReply {
                success: false,
                message: format!("No box found for conf: {devid}"),
            })
            .ok();
            return;
        }
    };
    s.join(devid.to_owned());
    s.extensions.insert::<Topic>(Topic {
        title: devid.to_owned(),
        tid: sid,
        // speaker: None,
    });

    // s.within(devid.to_owned()).emit("join", &s.id).ok();
    let rs = s.within(devid.to_owned()).sockets();
    let roomsids: HashSet<String> = HashSet::from_iter(rs.iter().map(|r| r.id.to_string()));
    info!("Configuring box: {} - {:?}", devid, roomsids);
    ack.send(&AckReply {
        success: true,
        message: format!("Configuring: {devid}"),
    })
    .ok();
}

pub async fn on_unset<A: Adapter>(
    s: SocketRef<A>,
    Data(devid): Data<String>,
    onlinedevs: State<OnlineDevs>,
    ack: AckSender,
) {
    let rsid = s.within(devid.to_owned()).sockets();
    if rsid.contains(&s) {
        // sending to all clients in the room (channel) except sender
        s.to(devid.to_owned()).emit("hangup", &s.id).await.ok();
        s.extensions.remove::<Topic>();
        s.leave(devid.to_owned());
        info!("{} has left room: {}", &s.id, devid);
    }

    if let Some(dev) = onlinedevs.get(&s.id).await {
        onlinedevs.remove(&s.id).await;
        onlinedevs.speaker_off(&dev).await;
        info!("Device unset: {dev}");
    }
    ack.send(&AckReply {
        success: true,
        message: format!("unset - {devid}"),
    })
    .ok();
}

pub async fn on_heartbeatping<A: Adapter>(s: SocketRef<A>) {
    if let Some(devroom) = s.extensions.get::<Topic>() {
        s.within(devroom.title)
            .emit("heartbeatpong", &())
            .await
            .ok();
    }
}

pub async fn on_checkdev<A: Adapter>(
    s: SocketRef<A>,
    Data(devs): Data<Vec<String>>,
    onlinedevs: State<OnlineDevs>,
) {
    if devs.len() > 0 {
        let mut devset = onlinedevs.val().await;
        devset.retain(|d| devs.contains(d));
        let mut devvec: Vec<String> = devset.into_iter().collect();
        devvec.sort();
        s.emit("onlinedev", &[devvec]).ok();
    }
}

pub async fn on_find<A: Adapter>(
    s: SocketRef<A>,
    Data(devid): Data<String>,
    onlinedevs: State<OnlineDevs>,
) {
    s.join(devid.to_owned());
    s.extensions.insert::<Topic>(Topic {
        title: devid.to_owned(),
        tid: s.id,
        // speaker: None,
    });
    onlinedevs.add(s.id, devid.to_owned()).await;
    onlinedevs.speaker_off(&devid).await;
    let rs = s.within(devid.to_owned()).sockets();
    let roomsids: HashSet<String> = HashSet::from_iter(rs.iter().map(|r| r.id.to_string()));
    info!("Camera online: {} - {:?}", devid, roomsids);
}

pub async fn on_watch<A: Adapter>(
    s: SocketRef<A>,
    Data(devid): Data<String>,
    onlinedevs: State<OnlineDevs>,
) {
    if !onlinedevs.val().await.contains(&devid) {
        s.emit("nodev", &Value::Null).ok();
        warn!("No device found for watching: {devid}");
        return;
    }

    let sid = match onlinedevs.getcamid(&devid).await {
        Some(s) => s,
        None => {
            warn!("No devid found for device: {devid}");
            return;
        }
    };

    s.join(devid.to_owned());

    s.extensions.insert::<Topic>(Topic {
        title: devid.to_owned(),
        tid: sid,
        // speaker: None,
    });

    s.within(devid.to_owned()).emit("join", &s.id).await.ok();
    let rs = s.within(devid.to_owned()).sockets();
    let roomsids: HashSet<String> = HashSet::from_iter(rs.iter().map(|r| r.id.to_string()));
    info!("Watching: {} - {:?}", devid, roomsids);
}

pub async fn on_speakerid<A: Adapter>(
    s: SocketRef<A>,
    Data(speaker): Data<String>,
    onlinedevs: State<OnlineDevs>,
) {
    let devname = match s.extensions.get::<Topic>() {
        Some(room) => room.title,
        None => {
            return;
        }
    };
    // sending to all clients in the room (channel) except sender
    if speaker.len() != 16 {
        onlinedevs.speaker_off(&devname).await;
        info!("Speakerid cleared: {}", devname);
        return;
    }
    match Sid::from_str(&speaker) {
        Ok(sid) => {
            info!("Speakerid updated: {} -> {}", speaker, devname);
            onlinedevs.speaker_on(sid, &devname).await;
        }
        Err(e) => {
            error!("Failed to parse Sid in `speaker` handler. {e:}");
        }
    };
}

pub async fn on_auth<A: Adapter>(s: SocketRef<A>) {
    // sending to all clients in the room (channel) except sender
    if let Some(devroom) = s.extensions.get::<Topic>() {
        s.to(devroom.title).emit("approve", &s.id).await.ok();
    }
}

pub async fn on_accept<A: Adapter>(s: SocketRef<A>, Data(pathid): Data<Value>) {
    // sending to all clients in 'device' room(channel), include sender
    if let Some(devroom) = s.extensions.get::<Topic>() {
        s.within(devroom.title).emit("bridge", &pathid).await.ok();
    }
}

pub async fn on_speech<A: Adapter>(
    s: SocketRef<A>,
    Data(sid): Data<Value>,
    onlinedevs: State<OnlineDevs>,
    ack: AckSender,
) {
    let devname = match s.extensions.get::<Topic>() {
        Some(room) => room.title,
        None => {
            return;
        }
    };
    // sending to all clients in the room (channel) except sender
    if onlinedevs.has_speaker(&devname).await {
        ack.send(&AckReply {
            success: false,
            message: "Failed",
        })
        .ok();
    } else {
        ack.send(&AckReply {
            success: false,
            message: "Failed",
        })
        .ok();
        s.to(devname).emit("speaking", &sid).await.ok();
    }
}

pub async fn on_hang<A: Adapter>(s: SocketRef<A>, Data(sid): Data<Value>) {
    // sending to all clients in the room (channel) except sender
    if let Some(devroom) = s.extensions.get::<Topic>() {
        s.to(devroom.title).emit("hangup", &sid).await.ok();
    }
}

pub async fn on_reject<A: Adapter>(s: SocketRef<A>) {
    if let Some(devroom) = s.extensions.get::<Topic>() {
        s.to(devroom.title).emit("full", &Value::Null).await.ok();
    }
}

pub async fn on_leave<A: Adapter>(s: SocketRef<A>, Data(devid): Data<String>) {
    let rsid = s.within(devid.to_owned()).sockets();
    if rsid.contains(&s) {
        // sending to all clients in the room (channel) except sender
        s.to(devid.to_owned()).emit("hangup", &s.id).await.ok();
        s.extensions.remove::<Topic>();
        s.leave(devid.to_owned());
        info!("{} has left room: {}", &s.id, devid);
    }
}
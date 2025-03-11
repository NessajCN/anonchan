mod handlers;
mod state;

use socketioxide::extract::SocketRef;

pub use state::{OnlineDevs, OnlineUsers};

pub async fn on_connect(socket: SocketRef) {
    socket.on_disconnect(handlers::on_disconnect);

    // OpenS1 part
    socket.on("signout", handlers::on_signout);
    socket.on("fetchAllUsers", handlers::on_fetchuser);
    // socket.on(
    //     "notify",
    //     |s: SocketRef, Data::<String>(quoted_author), Data::<String>(thread)| {
    //         // console.log(`quotedAuthor: ${quotedAuthor}`);
    //         s.to(quoted_author).emit("replyreminder", Raw(thread)).ok();
    //     },
    // );
    socket.on("identify", handlers::on_identify);

    // AI-box part
    socket.on("message", handlers::on_message);
    socket.on("checkbox", handlers::on_checkbox);
    socket.on("boxconf", handlers::on_boxconf);
    socket.on("unset", handlers::on_unset);

    // TJAI part

    // To be fixed: emit heartbeatpong with no payload
    socket.on("heartbeatping", handlers::on_heartbeatping);
    socket.on("checkdev", handlers::on_checkdev);
    socket.on("find", handlers::on_find);
    socket.on("watch", handlers::on_watch);
    socket.on("speakerid", handlers::on_speakerid);
    socket.on("auth", handlers::on_auth);
    socket.on("accept", handlers::on_accept);

    // Sendback acknowledgement to inform whether speaker is occupied.
    socket.on("speech", handlers::on_speech);
    socket.on("hang", handlers::on_hang);
    socket.on("reject", handlers::on_reject);
    socket.on("leave", handlers::on_leave);
}

use json::object;
use json::JsonValue;
use tokio::io::AsyncReadExt;
use tokio::io::AsyncWriteExt;
use tokio::sync::mpsc;
use tokio_listener::Connection;
use tokio_listener::ListenerAddress;

use clipboard::ClipboardContext;
use clipboard::ClipboardProvider;

#[derive(Debug)]
enum Action {
    Copying(String),
    Pasting,
}

async fn read_request_string(conn: &mut Connection) -> String {
    let buf = &mut [0 as u8; 1024];
    let mut rst_size = 0;
    let mut rst_vec: Vec<u8> = Vec::new();
    loop {
        let size = conn.read(buf).await.expect("read socket failed!");
        if size == 0 {
            break;
        }

        rst_vec.extend(buf.iter());
        rst_size += size;

        // we must ends with '\n'
        if buf[size - 1] == '\n' as u8 {
            break;
        }
    }

    if cfg!(debug_assertions) {
        eprintln!("{}", String::from_utf8(rst_vec.clone()).unwrap().trim());
    }

    String::from_utf8(rst_vec[0..rst_size].to_vec()).expect("Decode request string error!")
}

fn get_action(response: String) -> Option<Action> {
    match json::parse(&response) {
        Ok(rst) => {
            if !rst.has_key("type") {
                eprintln!("Invalid json format!");
                eprintln!("{}", rst);
                return None;
            }
            let type_str = &rst["type"];
            if *type_str == JsonValue::from("copy") {
                let contents = &rst["contents"];
                return Some(Action::Copying(contents.to_string()));
            }
            if *type_str == JsonValue::from("paste") {
                return Some(Action::Pasting);
            }

            None
        }
        Err(err) => {
            eprintln!("Parse request error: {}", err);
            None
        }
    }
}

fn compose_paste_response(content: &str) -> String {
    let response_json = object! {
        "type": "paste",
        "contents": content,
    };
    if cfg!(debug_assertions) {
        eprintln!("response: {}", response_json);
    }
    response_json.to_string()
}

#[tokio::main]
async fn main() {
    let addr: ListenerAddress = "0.0.0.0:33304".parse().unwrap();
    let sys_option = tokio_listener::SystemOptions::default();
    let user_option = tokio_listener::UserOptions::default();
    let mut l = tokio_listener::Listener::bind(&addr, &sys_option, &user_option)
        .await
        .unwrap();
    println!("Listening addr {}", addr);

    // 定义消息类型
    enum ClipboardMsg {
        Copy(String),
        Paste(tokio::sync::oneshot::Sender<String>),
    }

    // 创建专用剪贴板线程
    let (clip_tx, mut clip_rx) = mpsc::channel(32);
    std::thread::spawn(move || {
        let mut ctx = ClipboardContext::new().unwrap();
        while let Some(msg) = clip_rx.blocking_recv() {
            match msg {
                ClipboardMsg::Copy(text) => {
                    ctx.set_contents(text).unwrap();
                }
                ClipboardMsg::Paste(reply) => {
                    let content = ctx.get_contents().unwrap();
                    reply.send(content).ok();
                }
            }
        }
    });

    while let Ok((mut conn, _)) = l.accept().await {
        let clip_tx = clip_tx.clone();
        tokio::spawn(async move {
            let buf = read_request_string(&mut conn).await;
            let action = get_action(buf);

            match action {
                Some(Action::Copying(ss)) => {
                    // 发送复制请求
                    clip_tx.send(ClipboardMsg::Copy(ss)).await.unwrap();
                }
                Some(Action::Pasting) => {
                    // 创建 oneshot 通道接收回复
                    let (reply_tx, reply_rx) = tokio::sync::oneshot::channel();
                    clip_tx.send(ClipboardMsg::Paste(reply_tx)).await.unwrap();
                    let content = reply_rx.await.unwrap();
                    // 将内容写回 conn
                    conn.write_all(compose_paste_response(&content).as_bytes())
                        .await
                        .unwrap();
                }
                _ => eprintln!("Unsupported command!"),
            }

            conn.shutdown().await.unwrap();
        });
    }
}

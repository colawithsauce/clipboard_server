use json::object;
use json::JsonValue;
use tokio::io::AsyncReadExt;
use tokio::io::AsyncWriteExt;
use tokio::sync::Mutex;
use tokio_listener::Connection;
use tokio_listener::ListenerAddress;

use json;

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

        if cfg!(debug_assertions) {
            println!("{}", String::from_utf8(buf.to_vec()).unwrap());
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

fn handle_clients_copy(ss: String, ctx: &mut ClipboardContext) {
    match ctx.set_contents(ss) {
        Err(err) => {
            eprintln!("Error: {}", err);
        }
        _ => (),
    }
}

async fn handle_clients_pasting(conn: &mut Connection, ctx: &mut ClipboardContext) {
    let ss = ctx.get_contents().unwrap();
    let response_json = object! {
        "type": "paste",
        "contents": ss,
    };
    let response_bytes = response_json.to_string();

    if cfg!(debug_assertions) {
        println!("response: {}", response_json.to_string());
    }

    conn.write(response_bytes.as_bytes()).await.unwrap();
    conn.write("\n".as_bytes()).await.unwrap();
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
    while let Ok((mut conn, _)) = l.accept().await {
        let m = Mutex::new(ClipboardContext::new().unwrap());
        tokio::spawn(async move {
            let buf = read_request_string(&mut conn).await;
            let action = get_action(buf);
            match action {
                Some(Action::Copying(ss)) => {
                    let mut ctx = m.lock().await;
                    handle_clients_copy(ss, &mut ctx);
                }
                Some(Action::Pasting) => {
                    let mut ctx = m.lock().await;
                    handle_clients_pasting(&mut conn, &mut ctx).await;
                }
                _ => {
                    eprintln!("Unsupported command!")
                }
            }
        });
    }
}

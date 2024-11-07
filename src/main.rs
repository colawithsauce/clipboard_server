use json::JsonValue;
use tokio::io::AsyncReadExt;
use tokio::io::AsyncWriteExt;
use tokio::sync::Mutex;
use tokio_listener::ListenerAddress;

use json;

use clipboard::ClipboardContext;
use clipboard::ClipboardProvider;

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
            let mut buf = String::new();
            let size = conn.read_to_string(&mut buf).await.unwrap();
            println!("READ {} bytes contents from remote:\n {}", size, buf);

            match json::parse(&buf) {
                Ok(rst) => {
                    if !rst.has_key("type") {
                        eprintln!("Invalid json format!");
                        eprintln!("{}", rst);
                        return;
                    }

                    let type_str = &rst["type"];
                    if *type_str == JsonValue::from("copy") {
                        let mut ctx = m.lock().await;
                        match ctx.set_contents(rst["contents"].to_string()) {
                            Err(err) => {
                                eprintln!("Error: {}", err);
                            }
                            _ => (),
                        };
                    } else if *type_str == JsonValue::from("paste") {
                        let mut _buff = "".to_string();
                        {
                            let mut ctx = m.lock().await;
                            _buff = ctx
                                .get_contents()
                                .expect("Failed to get clipboard context!");
                        }
                        conn.write_all(_buff.as_bytes()).await.unwrap();
                    }
                }
                Err(err) => {
                    eprintln!("Error when parse json: {}", err);
                }
            }
        });
    }
}

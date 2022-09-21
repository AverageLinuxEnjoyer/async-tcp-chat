use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpStream,
};

#[tokio::main]
async fn main() -> tokio::io::Result<()> {
    let client = TcpStream::connect("127.0.0.1:6789").await?;
    let (mut rx, mut tx) = client.into_split();

    let input_task = tokio::spawn(async move {
        loop {
            let mut input = String::new();

            std::io::stdin()
                .read_line(&mut input)
                .expect("Line couldn't be read");

            tx.write_all(input.as_bytes()).await.unwrap();
        }
    });

    let listener_task = tokio::spawn(async move {
        loop {
            let mut buf = Vec::new();
            let _ = rx.read_buf(&mut buf).await.expect("Couldn't be read.");

            println!(
                "{}",
                String::from_utf8(buf).unwrap_or(String::from("<invalid data>"))
            );
        }
    });

    input_task.await?;
    listener_task.await?;

    Ok(())
}

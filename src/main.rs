use anyhow::{bail, Context as _, Result};
use bytes::{Buf, BufMut, BytesMut};
use futures::prelude::*;
use std::time::Duration;
use tokio::{
    io::{AsyncRead, AsyncWrite, BufWriter},
    prelude::*,
    time::timeout,
};
use tokio_serial::{Serial, SerialPortSettings};

use serialcat::{opt, prelude::*, util::GetChars};

#[tokio::main]
async fn main() {
    sc_main().await.unwrap_or_else(|e| {
        eprintln!("{:#}", e);
    });

    // Force stopping reading stdin
    std::process::exit(0);
}

async fn sc_main() -> Result<()> {
    let opt = opt::parse_args();

    let settings = SerialPortSettings {
        baud_rate: opt.baud_rate,
        data_bits: opt.data_bits,
        flow_control: opt.flow_control,
        parity: opt.parity,
        stop_bits: opt.stop_bits,
        timeout: Duration::from_millis(50),
    };
    let serial = Serial::from_path(&opt.port, &settings)
        .with_context(|| format!("Cannot open serial port: {}", opt.port))?;
    let (serial_rx, serial_tx) = tokio::io::split(serial);

    let reader = {
        let raw = opt.raw;
        async move {
            serial_reader(serial_rx, tokio::io::stdout(), raw)
                .await
                .context("An error occurred on reader")
        }
        .fuse()
    };
    let writer = {
        let escape_quit = opt.escape_quit;
        async move {
            serial_writer(tokio::io::stdin(), serial_tx, escape_quit)
                .await
                .context("An error occurred on writer")
        }
        .fuse()
    };
    futures::pin_mut!(reader, writer);

    futures::select! {
        result = &mut reader => result?,
        result = &mut writer => result?,
    }

    Ok(())
}

async fn serial_reader<R, W>(mut serial_rx: R, stdout: W, raw: bool) -> Result<()>
where
    R: AsyncRead + Unpin,
    W: AsyncWrite + Unpin,
{
    let mut buffer = BytesMut::with_capacity(1024);
    let mut stdout = BufWriter::new(stdout);

    let mut reversed = false;

    let drop_bufferd = timeout(Duration::from_millis(100), async {
        loop {
            let result = serial_rx
                .read_buf(&mut buffer)
                .await
                .context("Cannot read serial port");
            buffer.clear();

            if let Err(e) = result {
                break e;
            }
        }
    });
    if let Ok(e) = drop_bufferd.await {
        return Err(e);
    }

    loop {
        serial_rx
            .read_buf(&mut buffer)
            .await
            .context("Cannot read serial port")?;

        if raw {
            write_raw(&mut stdout, &mut buffer).await?;
        } else {
            write_visualized(&mut stdout, &mut buffer, &mut reversed).await?;
        }

        stdout.flush().await.context("Cannot flush stdout")?;
    }
}

async fn write_raw<W, B>(mut stdout: W, buffer: &mut B) -> Result<()>
where
    W: AsyncWrite + Unpin,
    B: Buf,
{
    while buffer.has_remaining() {
        let len = stdout
            .write_buf(buffer)
            .await
            .context("Cannot write stdout")?;
        if len == 0 {
            bail!("Cannot write stdout anymore");
        }
    }
    Ok(())
}

async fn write_slice<W>(mut stdout: W, mut buffer: &[u8]) -> Result<()>
where
    W: AsyncWrite + Unpin,
{
    while !buffer.is_empty() {
        let len = stdout.write(buffer).await.context("Cannot write stdout")?;
        buffer = &buffer[len..];
        if len == 0 {
            bail!("Cannot write stdout anymore");
        }
    }
    Ok(())
}

async fn write_visualized<W, B>(mut stdout: W, buffer: &mut B, reversed: &mut bool) -> Result<()>
where
    W: AsyncWrite + Unpin,
    B: Buf + BufMut,
{
    for ch in buffer.get_chars() {
        match ch {
            GetChars::Char(c) => {
                if c.is_control() && c != '\n' && c != '\t' {
                    if !*reversed {
                        write_slice(&mut stdout, b"\x1b[7m").await?;
                        *reversed = true;
                    }

                    if c < '\x20' {
                        write_slice(&mut stdout, b"^").await?;
                        write_slice(&mut stdout, &[c as u8 + b'@']).await?;
                    } else if c == '\x7f' {
                        write_slice(&mut stdout, b"^?").await?;
                    } else if c >= '\u{0080}' && c < '\u{00a0}' {
                        write_slice(&mut stdout, b"^[[").await?;
                        write_slice(&mut stdout, &[(c as u16 - 0x0080) as u8 + b'@']).await?;
                    } else {
                        unreachable!();
                    }
                } else {
                    if *reversed {
                        write_slice(&mut stdout, b"\x1b[m").await?;
                        *reversed = false;
                    }

                    let mut b = [0; 4];
                    write_slice(&mut stdout, c.encode_utf8(&mut b).as_bytes()).await?;
                }
            }
            GetChars::Err(b) => {
                if !*reversed {
                    write_slice(&mut stdout, b"\x1b[7m").await?;
                    *reversed = true;
                }

                write_slice(&mut stdout, b"<").await?;
                let s = format!("{:02X}", b);
                write_slice(&mut stdout, s.as_bytes()).await?;
                write_slice(&mut stdout, b">").await?;
            }
        }
    }

    Ok(())
}

async fn serial_writer<R, W>(mut stdin: R, mut serial_tx: W, escape_quit: bool) -> Result<()>
where
    R: AsyncRead + Unpin,
    W: AsyncWrite + Unpin,
{
    let mut buffer = BytesMut::with_capacity(1024);

    loop {
        stdin
            .read_buf(&mut buffer)
            .await
            .context("Cannot read stdin")?;

        if !buffer.has_remaining() {
            // EOF
            if escape_quit {
                return Ok(());
            }
        }

        while buffer.has_remaining() {
            let len = serial_tx
                .write_buf(&mut buffer)
                .await
                .context("Cannot write serial port")?;
            if len == 0 {
                bail!("Cannot write serial port anymore");
            }
        }

        serial_tx
            .flush()
            .await
            .context("Cannot flush serial port")?;
    }
}

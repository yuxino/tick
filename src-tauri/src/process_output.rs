use std::io;
use std::process::{ExitStatus, Stdio};
use tokio::io::{AsyncRead, AsyncReadExt};
use tokio::process::Command;
use tokio::time::{timeout, Duration};

#[derive(Default)]
pub struct CapturedStream {
    pub bytes: Vec<u8>,
    pub truncated: bool,
}

pub struct CapturedOutput {
    pub stdout: CapturedStream,
    pub stderr: CapturedStream,
    pub status: Option<ExitStatus>,
    pub timed_out: bool,
}

async fn drain_capped(
    mut reader: impl AsyncRead + Unpin,
    output: &mut CapturedStream,
    limit: usize,
) -> io::Result<()> {
    let mut buffer = [0_u8; 8192];
    loop {
        let count = reader.read(&mut buffer).await?;
        if count == 0 {
            return Ok(());
        }
        let retained = count.min(limit.saturating_sub(output.bytes.len()));
        output.bytes.extend_from_slice(&buffer[..retained]);
        output.truncated |= retained < count;
        // Continue draining even at the cap so a noisy process cannot deadlock on its pipe.
    }
}

pub async fn capture_output(
    command: &mut Command,
    duration: Duration,
    limit: usize,
) -> io::Result<CapturedOutput> {
    let mut child = command
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true)
        .spawn()?;
    let mut stdout = CapturedStream::default();
    let mut stderr = CapturedStream::default();
    let result = timeout(duration, async {
        tokio::try_join!(
            drain_capped(
                child.stdout.take().expect("piped stdout"),
                &mut stdout,
                limit
            ),
            drain_capped(
                child.stderr.take().expect("piped stderr"),
                &mut stderr,
                limit
            ),
            child.wait(),
        )
    })
    .await;
    let (status, timed_out) = match result {
        Ok(Ok(((), (), status))) => (Some(status), false),
        Ok(Err(error)) => {
            let _ = child.kill().await;
            return Err(error);
        }
        Err(_) => {
            // Reap the direct child before returning; preserve output captured before the deadline.
            let _ = child.kill().await;
            (None, true)
        }
    };
    Ok(CapturedOutput {
        stdout,
        stderr,
        status,
        timed_out,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn caps_both_streams_without_blocking_the_child() {
        let mut command = Command::new("node");
        command.args([
            "-e",
            "process.stdout.write('x'.repeat(1048576)); process.stderr.write('y'.repeat(1048576));",
        ]);
        let output = capture_output(&mut command, Duration::from_secs(10), 1024)
            .await
            .unwrap();
        assert!(output.status.unwrap().success());
        assert!(!output.timed_out);
        assert_eq!(output.stdout.bytes, vec![b'x'; 1024]);
        assert_eq!(output.stderr.bytes, vec![b'y'; 1024]);
        assert!(output.stdout.truncated && output.stderr.truncated);
    }

    #[tokio::test]
    async fn preserves_partial_output_on_timeout() {
        let mut command = Command::new("node");
        command.args([
            "-e",
            "console.log('started ' + process.pid); setInterval(() => {}, 1000);",
        ]);
        let output = capture_output(&mut command, Duration::from_secs(2), 1024)
            .await
            .unwrap();
        assert!(output.timed_out);
        assert!(output.status.is_none());
        assert!(String::from_utf8_lossy(&output.stdout.bytes).contains("started"));
        #[cfg(target_os = "macos")]
        {
            let text = String::from_utf8_lossy(&output.stdout.bytes);
            let pid = text
                .split_whitespace()
                .nth(1)
                .unwrap()
                .parse::<libc::pid_t>()
                .unwrap();
            assert_eq!(unsafe { libc::kill(pid, 0) }, -1);
            assert_eq!(
                std::io::Error::last_os_error().raw_os_error(),
                Some(libc::ESRCH)
            );
        }
    }

    #[tokio::test]
    async fn reports_spawn_failure() {
        let mut command = Command::new("tick-nonexistent-debug-program");
        assert!(capture_output(&mut command, Duration::from_secs(1), 1024)
            .await
            .is_err());
    }
}

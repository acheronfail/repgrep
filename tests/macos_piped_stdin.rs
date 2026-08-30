#![cfg(target_os = "macos")]

use std::fs::File;
use std::io::{Read, Write};
use std::os::fd::{FromRawFd, RawFd};
use std::os::raw::{c_char, c_int, c_ulong, c_ushort, c_void};
use std::os::unix::process::CommandExt;
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

const TIOCSCTTY: c_ulong = 0x2000_7461;

#[repr(C)]
struct WindowSize {
    rows: c_ushort,
    columns: c_ushort,
    x_pixels: c_ushort,
    y_pixels: c_ushort,
}

unsafe extern "C" {
    fn openpty(
        master: *mut c_int,
        slave: *mut c_int,
        name: *mut c_char,
        termios: *const c_void,
        window_size: *const WindowSize,
    ) -> c_int;
    fn setsid() -> c_int;
    fn ioctl(fd: c_int, request: c_ulong, ...) -> c_int;
}

fn pseudo_terminal() -> (File, File) {
    let mut master: RawFd = -1;
    let mut slave: RawFd = -1;
    let window_size = WindowSize {
        rows: 24,
        columns: 80,
        x_pixels: 0,
        y_pixels: 0,
    };

    // SAFETY: openpty initializes both file descriptors on success, and the
    // returned descriptors are immediately wrapped in owning File values.
    let result = unsafe {
        openpty(
            &mut master,
            &mut slave,
            std::ptr::null_mut(),
            std::ptr::null(),
            &window_size,
        )
    };
    assert_eq!(result, 0, "failed to open a pseudo-terminal");

    // SAFETY: openpty returned two new, valid, independently owned fds.
    unsafe { (File::from_raw_fd(master), File::from_raw_fd(slave)) }
}

#[test]
fn reads_ui_events_from_tty_when_stdin_is_a_pipe() {
    let (master, slave) = pseudo_terminal();
    let terminal_input = Arc::new(Mutex::new(master.try_clone().unwrap()));
    let terminal_input_for_reader = Arc::clone(&terminal_input);
    let ui_rendered = Arc::new(AtomicBool::new(false));
    let ui_rendered_for_reader = Arc::clone(&ui_rendered);
    let mut terminal_output = master;

    let mut command = Command::new(env!("CARGO_BIN_EXE_rgr"));
    command
        .arg(r"\+00:00")
        .stdin(Stdio::piped())
        .stdout(Stdio::from(slave.try_clone().unwrap()))
        .stderr(Stdio::from(slave));

    // Give the child a controlling terminal while leaving stdin connected to
    // a pipe, matching invocations such as `... | xargs rgr ...`.
    // SAFETY: this runs after fork and before exec; it calls only async-signal-
    // safe syscalls and reports their errors through Command's normal path.
    unsafe {
        command.pre_exec(|| {
            if setsid() == -1 || ioctl(1, TIOCSCTTY, 0 as c_int) == -1 {
                return Err(std::io::Error::last_os_error());
            }
            Ok(())
        });
    }

    let mut child = command.spawn().expect("failed to start repgrep");
    child
        .stdin
        .take()
        .expect("repgrep stdin should be piped")
        .write_all(b"2023-07-19T11:43:45.012305+00:00 [trace] match\n")
        .expect("failed to write search input");

    // A pseudo-terminal is a kernel device, not a terminal emulator. Answer
    // cursor-position queries emitted while Crossterm initializes the terminal.
    let output_reader = thread::spawn(move || {
        let mut output = Vec::new();
        let mut responses = 0;
        let mut chunk = [0; 1024];
        loop {
            match terminal_output.read(&mut chunk) {
                Ok(0) => break,
                Ok(read) => output.extend_from_slice(&chunk[..read]),
                Err(error) if error.raw_os_error() == Some(5) => break,
                Err(error) => panic!("failed to read pseudo-terminal output: {error}"),
            }

            if output
                .windows(b"2023-07-19".len())
                .any(|window| window == b"2023-07-19")
            {
                ui_rendered_for_reader.store(true, Ordering::Release);
            }

            let queries = output
                .windows(4)
                .filter(|window| *window == b"\x1b[6n")
                .count();
            while responses < queries {
                terminal_input_for_reader
                    .lock()
                    .unwrap()
                    .write_all(b"\x1b[1;1R")
                    .expect("failed to answer cursor-position query");
                responses += 1;
            }
        }
        output
    });

    // Wait for a rendered match before sending input. This catches regressions
    // where the first key press is required to unblock terminal initialization.
    let render_deadline = Instant::now() + Duration::from_secs(5);
    while !ui_rendered.load(Ordering::Acquire) {
        if let Some(status) = child.try_wait().expect("failed to wait for repgrep") {
            drop(terminal_input);
            let output = output_reader.join().expect("output reader thread panicked");
            let terminal_output = String::from_utf8_lossy(&output);
            panic!("repgrep exited before rendering the UI ({status}):\n{terminal_output}");
        }
        if Instant::now() >= render_deadline {
            child.kill().expect("failed to stop hung repgrep");
            child.wait().expect("failed to reap hung repgrep");
            drop(terminal_input);
            let output = output_reader.join().expect("output reader thread panicked");
            let terminal_output = String::from_utf8_lossy(&output);
            panic!("repgrep did not render before receiving input:\n{terminal_output}");
        }
        thread::sleep(Duration::from_millis(20));
    }

    terminal_input
        .lock()
        .unwrap()
        .write_all(b"q")
        .expect("failed to send quit event");

    let deadline = Instant::now() + Duration::from_secs(10);
    let mut timed_out = false;
    while child
        .try_wait()
        .expect("failed to wait for repgrep")
        .is_none()
    {
        if Instant::now() >= deadline {
            child.kill().expect("failed to stop hung repgrep");
            child.wait().expect("failed to reap hung repgrep");
            timed_out = true;
            break;
        }
        thread::sleep(Duration::from_millis(20));
    }
    drop(terminal_input);

    let output = output_reader.join().expect("output reader thread panicked");
    let terminal_output = String::from_utf8_lossy(&output);

    assert!(
        !terminal_output.contains("Failed to initialize input reader"),
        "repgrep failed to read events from the controlling terminal:\n{terminal_output}"
    );
    assert!(
        !terminal_output.contains("Failed to restore terminal state"),
        "repgrep failed to restore the terminal:\n{terminal_output}"
    );
    assert!(
        !timed_out,
        "repgrep did not exit within 10 seconds:\n{terminal_output}"
    );
    assert!(
        terminal_output.contains("Cancelled"),
        "repgrep did not process the quit event:\n{terminal_output}"
    );
}

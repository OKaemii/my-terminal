use anyhow::{Context, Result};
use portable_pty::{native_pty_system, CommandBuilder, PtySize};
use std::io::{Read, Write};
use std::sync::mpsc;
use std::thread;

pub const PROMPT_MARKER: &str = "__MYTERM_READY__";
pub const META_MARKER: &str = "__MYTERM_META__:";

pub struct PtySession {
    pub writer: Box<dyn Write + Send>,
    pub rx: mpsc::Receiver<Vec<u8>>,
    pub master: Box<dyn portable_pty::MasterPty + Send>,
    _child: Box<dyn portable_pty::Child + Send>,
}

impl PtySession {
    pub fn spawn(rows: u16, cols: u16) -> Result<Self> {
        let pty_system = native_pty_system();
        let size = PtySize {
            rows,
            cols,
            pixel_width: 0,
            pixel_height: 0,
        };
        let pair = pty_system.openpty(size).context("openpty failed")?;

        let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/bash".to_string());
        let shell_name = std::path::Path::new(&shell)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("bash")
            .to_string();

        let mut cmd = CommandBuilder::new(&shell);
        cmd.env("TERM", "xterm-256color");
        cmd.env("MYTERM", "1");
        cmd.env("COLORTERM", "truecolor");

        let child = pair
            .slave
            .spawn_command(cmd)
            .context("spawn shell failed")?;
        drop(pair.slave);

        let (tx, rx) = mpsc::channel::<Vec<u8>>();
        let mut reader = pair
            .master
            .try_clone_reader()
            .context("clone reader failed")?;

        thread::spawn(move || {
            let mut buf = [0u8; 4096];
            loop {
                match reader.read(&mut buf) {
                    Ok(0) | Err(_) => break,
                    Ok(n) => {
                        if tx.send(buf[..n].to_vec()).is_err() {
                            break;
                        }
                    }
                }
            }
        });

        let writer = pair.master.take_writer().context("take writer failed")?;
        let mut session = Self {
            writer,
            rx,
            master: pair.master,
            _child: child,
        };

        let init = Self::init_script(&shell_name);
        session.send_raw(&init)?;

        Ok(session)
    }

    pub fn init_script(shell: &str) -> String {
        match shell {
            "zsh" => format!(
                "stty -echo 2>/dev/null; unsetopt PROMPT_CR 2>/dev/null; precmd() {{ printf '{meta}%s:%d\\n' \"$PWD\" \"$?\"; printf '{prompt}\\n'; }}; PS1=''; RPROMPT=''; printf '\\n'\n",
                meta = META_MARKER,
                prompt = PROMPT_MARKER,
            ),
            _ => format!(
                "stty -echo 2>/dev/null; PROMPT_COMMAND='printf \"{meta}%s:%d\\n\" \"$PWD\" \"$?\"'; PS1='{prompt}\\n'; printf '\\n'\n",
                meta = META_MARKER,
                prompt = PROMPT_MARKER,
            ),
        }
    }

    pub fn send_command(&mut self, cmd: &str) -> Result<()> {
        writeln!(self.writer, "{cmd}")?;
        self.writer.flush()?;
        Ok(())
    }

    pub fn send_raw(&mut self, s: &str) -> Result<()> {
        self.writer.write_all(s.as_bytes())?;
        self.writer.flush()?;
        Ok(())
    }

    pub fn send_resize(&self, rows: u16, cols: u16) -> Result<()> {
        self.master
            .resize(PtySize {
                rows,
                cols,
                pixel_width: 0,
                pixel_height: 0,
            })
            .context("resize failed")?;
        Ok(())
    }

    pub fn send_signal(&mut self, sig: &str) -> Result<()> {
        match sig {
            "int" => self.send_raw("\x03"),
            "eof" => self.send_raw("\x04"),
            _ => Ok(()),
        }
    }
}

#[cfg(test)]
#[path = "pty_tests.rs"]
mod tests;

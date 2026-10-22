use super::forwarding::{RemoteForwardTask, bind_first, run_remote_forward};
use super::*;

pub(super) struct FixtureHandler {
    authorized_keys: Vec<ssh_key::PublicKey>,
    passwords: HashMap<String, String>,
    commands: HashMap<String, CommandResponse>,
    authentication_delay: Option<Duration>,
    channel_open_delay: Option<Duration>,
    pub(super) events: Arc<Mutex<Vec<SshEvent>>>,
    sftp_root: PathBuf,
    channels: HashMap<ChannelId, Channel<Msg>>,
    shells: HashSet<ChannelId>,
    shell_buffers: HashMap<ChannelId, Vec<u8>>,
    remote_forwards: HashMap<(String, u32), RemoteForwardTask>,
    pub(super) tasks: ChildTaskTracker,
    task_probe: SshTaskProbe,
    #[cfg(test)]
    never_finish_child_drop_delay: Option<Duration>,
}

impl FixtureHandler {
    pub(super) fn new(
        behavior: FixtureBehavior,
        events: Arc<Mutex<Vec<SshEvent>>>,
        sftp_root: PathBuf,
        task_probe: SshTaskProbe,
        #[cfg(test)] never_finish_child_drop_delay: Option<Duration>,
    ) -> Self {
        let tasks = ChildTaskTracker::new(&task_probe);
        let FixtureBehavior {
            authorized_keys,
            passwords,
            commands,
            authentication_delay,
            channel_open_delay,
        } = behavior;
        Self {
            authorized_keys,
            passwords,
            commands,
            authentication_delay,
            channel_open_delay,
            events,
            sftp_root,
            channels: HashMap::new(),
            shells: HashSet::new(),
            shell_buffers: HashMap::new(),
            remote_forwards: HashMap::new(),
            tasks,
            task_probe,
            #[cfg(test)]
            never_finish_child_drop_delay,
        }
    }

    pub(super) fn clone_for_session(&self) -> Self {
        Self::new(
            FixtureBehavior {
                authorized_keys: self.authorized_keys.clone(),
                passwords: self.passwords.clone(),
                commands: self.commands.clone(),
                authentication_delay: self.authentication_delay,
                channel_open_delay: self.channel_open_delay,
            },
            Arc::clone(&self.events),
            self.sftp_root.clone(),
            self.task_probe.clone(),
            #[cfg(test)]
            self.never_finish_child_drop_delay,
        )
    }
}

impl server::Handler for FixtureHandler {
    type Error = russh::Error;

    async fn auth_publickey(
        &mut self,
        user: &str,
        public_key: &ssh_key::PublicKey,
    ) -> Result<server::Auth, Self::Error> {
        if let Some(delay) = self.authentication_delay {
            tokio::time::sleep(delay).await;
        }
        let accepted = self.authorized_keys.iter().any(|key| key == public_key);
        record(
            &self.events,
            SshEvent::PublicKeyAuth {
                user: user.to_owned(),
                fingerprint: public_key.fingerprint(ssh_key::HashAlg::Sha256).to_string(),
                accepted,
            },
        );
        Ok(if accepted {
            server::Auth::Accept
        } else {
            server::Auth::reject()
        })
    }

    async fn auth_password(
        &mut self,
        user: &str,
        password: &str,
    ) -> Result<server::Auth, Self::Error> {
        if let Some(delay) = self.authentication_delay {
            tokio::time::sleep(delay).await;
        }
        Ok(
            if self
                .passwords
                .get(user)
                .is_some_and(|expected| expected == password)
            {
                server::Auth::Accept
            } else {
                server::Auth::reject()
            },
        )
    }

    async fn channel_open_session(
        &mut self,
        channel: Channel<Msg>,
        _session: &mut Session,
    ) -> Result<bool, Self::Error> {
        if let Some(delay) = self.channel_open_delay {
            tokio::time::sleep(delay).await;
        }
        record(&self.events, SshEvent::SessionOpened);
        self.channels.insert(channel.id(), channel);
        #[cfg(test)]
        if let Some(drop_delay) = self.never_finish_child_drop_delay.take() {
            let _ = self.tasks.spawn(NeverFinishChild { drop_delay });
        }
        Ok(true)
    }

    async fn channel_close(
        &mut self,
        channel: ChannelId,
        _session: &mut Session,
    ) -> Result<(), Self::Error> {
        self.channels.remove(&channel);
        self.shells.remove(&channel);
        self.shell_buffers.remove(&channel);
        Ok(())
    }

    async fn pty_request(
        &mut self,
        channel: ChannelId,
        term: &str,
        col_width: u32,
        row_height: u32,
        pix_width: u32,
        pix_height: u32,
        _modes: &[(Pty, u32)],
        session: &mut Session,
    ) -> Result<(), Self::Error> {
        record(
            &self.events,
            SshEvent::Pty {
                term: term.to_owned(),
                columns: col_width,
                rows: row_height,
                pixel_width: pix_width,
                pixel_height: pix_height,
            },
        );
        session.channel_success(channel)?;
        Ok(())
    }

    async fn shell_request(
        &mut self,
        channel: ChannelId,
        session: &mut Session,
    ) -> Result<(), Self::Error> {
        record(&self.events, SshEvent::Shell);
        self.shells.insert(channel);
        self.shell_buffers.entry(channel).or_default();
        session.channel_success(channel)?;
        Ok(())
    }

    async fn exec_request(
        &mut self,
        channel: ChannelId,
        data: &[u8],
        session: &mut Session,
    ) -> Result<(), Self::Error> {
        let command = String::from_utf8_lossy(data).into_owned();
        if let Some(request) = parse_scp_request(&command) {
            let accepted = request.is_ok();
            record(&self.events, SshEvent::Exec { command, accepted });
            if let Some(channel) = self.channels.remove(&channel) {
                session.channel_success(channel.id())?;
                let root = self.sftp_root.clone();
                let _ = self.tasks.spawn(async move {
                    serve_scp(channel, root, request).await;
                });
            } else {
                session.channel_failure(channel)?;
            }
            return Ok(());
        }
        let outcome = configured_command(&command, &self.commands);
        record(
            &self.events,
            SshEvent::Exec {
                command,
                accepted: outcome.accepted,
            },
        );
        finish_command(channel, outcome, session)
    }

    async fn data(
        &mut self,
        channel: ChannelId,
        data: &[u8],
        session: &mut Session,
    ) -> Result<(), Self::Error> {
        if !self.shells.contains(&channel) {
            return Ok(());
        }
        let buffer = self.shell_buffers.entry(channel).or_default();
        if buffer.len().saturating_add(data.len()) > MAX_SHELL_COMMAND {
            buffer.clear();
            return finish_command(channel, CommandOutcome::rejected(), session);
        }
        buffer.extend_from_slice(data);
        while let Some(newline) = buffer.iter().position(|byte| *byte == b'\n') {
            let mut line = buffer.drain(..=newline).collect::<Vec<_>>();
            while matches!(line.last(), Some(b'\n' | b'\r')) {
                line.pop();
            }
            let command = String::from_utf8_lossy(&line).into_owned();
            let outcome = configured_command(&command, &self.commands);
            record(
                &self.events,
                SshEvent::Exec {
                    command,
                    accepted: outcome.accepted,
                },
            );
            let closes = outcome.closes;
            finish_command(channel, outcome, session)?;
            if closes {
                self.shells.remove(&channel);
                break;
            }
        }
        Ok(())
    }

    async fn window_change_request(
        &mut self,
        _channel: ChannelId,
        col_width: u32,
        row_height: u32,
        pix_width: u32,
        pix_height: u32,
        _session: &mut Session,
    ) -> Result<(), Self::Error> {
        record(
            &self.events,
            SshEvent::Resize {
                columns: col_width,
                rows: row_height,
                pixel_width: pix_width,
                pixel_height: pix_height,
            },
        );
        Ok(())
    }

    async fn agent_request(
        &mut self,
        channel: ChannelId,
        session: &mut Session,
    ) -> Result<bool, Self::Error> {
        // Agent protocol injection is provided out of band by `AgentFixture`.
        // Do not claim support for OpenSSH agent forwarding unless the fixture
        // can service the forwarded channel end to end.
        record(&self.events, SshEvent::AgentForward { accepted: false });
        session.channel_failure(channel)?;
        Ok(false)
    }

    async fn subsystem_request(
        &mut self,
        channel: ChannelId,
        name: &str,
        session: &mut Session,
    ) -> Result<(), Self::Error> {
        let accepted = name == "sftp" && self.channels.contains_key(&channel);
        record(
            &self.events,
            SshEvent::Subsystem {
                name: name.to_owned(),
                accepted,
            },
        );
        if accepted {
            let channel = self
                .channels
                .remove(&channel)
                .ok_or(russh::Error::Inconsistent)?;
            let sftp = SandboxedSftpSession::new(&self.sftp_root);
            session.channel_success(channel.id())?;
            let _ = self.tasks.spawn(async move {
                russh_sftp::server::run(channel.into_stream(), sftp).await;
            });
        } else {
            session.channel_failure(channel)?;
        }
        Ok(())
    }

    async fn channel_open_direct_tcpip(
        &mut self,
        channel: Channel<Msg>,
        host_to_connect: &str,
        port_to_connect: u32,
        _originator_address: &str,
        _originator_port: u32,
        _session: &mut Session,
    ) -> Result<bool, Self::Error> {
        let endpoint = u16::try_from(port_to_connect)
            .ok()
            .and_then(|port| LoopbackEndpoint::new(host_to_connect, port).ok());
        let connection = match endpoint {
            Some(endpoint) => tokio::net::TcpStream::connect(endpoint.socket_addrs())
                .await
                .ok(),
            None => None,
        };
        let accepted = connection.is_some();
        record(
            &self.events,
            SshEvent::DirectTcpip {
                target: host_to_connect.to_owned(),
                port: port_to_connect,
                accepted,
            },
        );
        if let Some(mut target) = connection {
            let _ = self.tasks.spawn(async move {
                let mut channel = channel.into_stream();
                let _ = tokio::io::copy_bidirectional(&mut channel, &mut target).await;
                let _ = channel.shutdown().await;
            });
        }
        Ok(accepted)
    }

    async fn tcpip_forward(
        &mut self,
        address: &str,
        port: &mut u32,
        session: &mut Session,
    ) -> Result<bool, Self::Error> {
        let requested_port = *port;
        let endpoint = u16::try_from(requested_port)
            .ok()
            .and_then(|port| LoopbackEndpoint::new(address, port).ok());
        let Some(endpoint) = endpoint else {
            record(
                &self.events,
                SshEvent::RemoteForward {
                    address: address.to_owned(),
                    port: requested_port,
                    accepted: false,
                },
            );
            return Ok(false);
        };
        let listener = bind_first(endpoint.socket_addrs()).await;
        let Ok(listener) = listener else {
            record(
                &self.events,
                SshEvent::RemoteForward {
                    address: address.to_owned(),
                    port: requested_port,
                    accepted: false,
                },
            );
            return Ok(false);
        };
        let allocated = u32::from(listener.local_addr()?.port());
        *port = allocated;
        let (cancel, cancel_rx) = watch::channel(false);
        let (completion_sender, completion) = tokio::sync::oneshot::channel();
        let forward_address = address.to_owned();
        let session_handle = session.handle();
        let Some(abort) = self.tasks.spawn(async move {
            run_remote_forward(
                listener,
                session_handle,
                forward_address,
                allocated,
                cancel_rx,
            )
            .await;
            let _ = completion_sender.send(());
        }) else {
            *port = requested_port;
            record(
                &self.events,
                SshEvent::RemoteForward {
                    address: address.to_owned(),
                    port: requested_port,
                    accepted: false,
                },
            );
            return Ok(false);
        };
        self.remote_forwards.insert(
            (address.to_owned(), allocated),
            RemoteForwardTask {
                cancel,
                abort,
                completion,
            },
        );
        record(
            &self.events,
            SshEvent::RemoteForward {
                address: address.to_owned(),
                port: allocated,
                accepted: true,
            },
        );
        Ok(true)
    }

    async fn cancel_tcpip_forward(
        &mut self,
        address: &str,
        port: u32,
        _session: &mut Session,
    ) -> Result<bool, Self::Error> {
        let task = self.remote_forwards.remove(&(address.to_owned(), port));
        let accepted = task.is_some();
        if let Some(task) = task {
            task.stop().await;
        }
        record(
            &self.events,
            SshEvent::RemoteForwardCancelled {
                address: address.to_owned(),
                port,
                accepted,
            },
        );
        Ok(accepted)
    }
}

struct CommandOutcome {
    accepted: bool,
    stdout: Vec<u8>,
    stderr: Vec<u8>,
    termination: CommandTermination,
    closes: bool,
}

impl CommandOutcome {
    fn rejected() -> Self {
        Self {
            accepted: false,
            stdout: Vec::new(),
            stderr: b"command rejected by hermetic SSH fixture\n".to_vec(),
            termination: CommandTermination::Status(126),
            closes: true,
        }
    }
}

fn configured_command(
    command: &str,
    commands: &HashMap<String, CommandResponse>,
) -> CommandOutcome {
    if let Some(response) = commands.get(command) {
        return CommandOutcome {
            accepted: true,
            stdout: response.stdout.clone(),
            stderr: response.stderr.clone(),
            termination: response.termination.clone(),
            closes: true,
        };
    }
    if let Some(marker) = command.strip_prefix("rterm-test-marker ")
        && marker.len() <= 1024
        && !marker.chars().any(char::is_control)
    {
        return CommandOutcome {
            accepted: true,
            stdout: marker.as_bytes().to_vec(),
            stderr: Vec::new(),
            termination: CommandTermination::Status(0),
            closes: true,
        };
    }
    if let Some(message) = command.strip_prefix("echo ")
        && message.len() <= 1024
        && !message.chars().any(char::is_control)
    {
        let mut stdout = message.as_bytes().to_vec();
        stdout.push(b'\n');
        return CommandOutcome {
            accepted: true,
            stdout,
            stderr: Vec::new(),
            termination: CommandTermination::Status(0),
            closes: true,
        };
    }
    if let Some(code) = command.strip_prefix("exit ")
        && let Ok(code) = code.parse::<u8>()
    {
        return CommandOutcome {
            accepted: true,
            stdout: Vec::new(),
            stderr: Vec::new(),
            termination: CommandTermination::Status(u32::from(code)),
            closes: true,
        };
    }
    CommandOutcome::rejected()
}

fn finish_command(
    channel: ChannelId,
    outcome: CommandOutcome,
    session: &mut Session,
) -> Result<(), russh::Error> {
    if outcome.accepted {
        session.channel_success(channel)?;
    } else {
        session.channel_failure(channel)?;
    }
    if !outcome.stdout.is_empty() {
        session.data(channel, outcome.stdout)?;
    }
    if !outcome.stderr.is_empty() {
        session.extended_data(channel, 1, outcome.stderr)?;
    }
    match outcome.termination {
        CommandTermination::Status(status) => session.exit_status_request(channel, status)?,
        CommandTermination::Stall => return Ok(()),
        CommandTermination::Signal {
            signal,
            core_dumped,
            error_message,
        } => session.exit_signal_request(channel, signal, core_dumped, &error_message, "")?,
    }
    session.eof(channel)?;
    session.close(channel)?;
    Ok(())
}

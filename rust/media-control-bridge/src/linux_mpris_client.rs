use anyhow::Result;

#[cfg(target_os = "linux")]
pub async fn run_mpris_client(
    connect: String,
    token: Option<String>,
    name: String,
    poll_secs: u64,
) -> Result<()> {
    use mpris_server::Player;
    use std::time::Duration;
    use tokio::sync::mpsc;

    use crate::net::send_command;
    use crate::protocol::MediaCommand;

    let bus_name = format!("rs.mediabridge.{}", sanitize_bus_part(&name));
    let player = Player::builder(&bus_name)
        .can_play(true)
        .can_pause(true)
        .can_go_next(true)
        .can_go_previous(true)
        .can_control(true)
        .identity(name.clone())
        .build()
        .await?;

    let (tx, mut rx) = mpsc::unbounded_channel::<MediaCommand>();

    {
        let tx = tx.clone();
        player.connect_play_pause(move |_| {
            let _ = tx.send(MediaCommand::PlayPause);
        });
    }
    {
        let tx = tx.clone();
        player.connect_play(move |_| {
            let _ = tx.send(MediaCommand::Play);
        });
    }
    {
        let tx = tx.clone();
        player.connect_pause(move |_| {
            let _ = tx.send(MediaCommand::Pause);
        });
    }
    {
        let tx = tx.clone();
        player.connect_stop(move |_| {
            let _ = tx.send(MediaCommand::Stop);
        });
    }
    {
        let tx = tx.clone();
        player.connect_next(move |_| {
            let _ = tx.send(MediaCommand::Next);
        });
    }
    {
        let tx = tx;
        player.connect_previous(move |_| {
            let _ = tx.send(MediaCommand::Previous);
        });
    }

    eprintln!("MPRIS client registered as org.mpris.MediaPlayer2.{bus_name}");
    eprintln!("Connected controls will be sent to {connect}");

    let run_fut = player.run();
    tokio::pin!(run_fut);

    // poll_secs == 0 disables status polling; the interval then never fires.
    let mut poll = tokio::time::interval(Duration::from_secs(poll_secs.max(1)));
    poll.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
    let mut last: Option<StateKey> = None;
    let mut status_warned = false;

    loop {
        tokio::select! {
            _ = &mut run_fut => {
                return Ok(());
            }
            maybe_cmd = rx.recv() => {
                match maybe_cmd {
                    Some(cmd) => {
                        if let Err(err) = send_command(&connect, token.as_deref(), cmd).await {
                            eprintln!("send failed: {err:#}");
                        }
                        if poll_secs > 0 {
                            // Give the player a moment to act, then refresh soon.
                            poll.reset_after(Duration::from_millis(300));
                        }
                    }
                    None => return Ok(()),
                }
            }
            _ = poll.tick(), if poll_secs > 0 => {
                let state = match send_command(&connect, token.as_deref(), MediaCommand::Status).await {
                    Ok(state) => {
                        status_warned = false;
                        state.unwrap_or_default()
                    }
                    Err(err) => {
                        if !status_warned {
                            eprintln!("status failed (will keep retrying quietly): {err:#}");
                            status_warned = true;
                        }
                        Default::default()
                    }
                };
                let key = StateKey::from(&state);
                if last.as_ref() != Some(&key) {
                    if let Err(err) = apply_state(&player, &key).await {
                        eprintln!("updating MPRIS state failed: {err:#}");
                    }
                    last = Some(key);
                }
            }
        }
    }
}

#[cfg(target_os = "linux")]
#[derive(Debug, PartialEq, Eq)]
struct StateKey {
    status: mpris_server::PlaybackStatus,
    title: Option<String>,
    artist: Option<String>,
    album: Option<String>,
}

#[cfg(target_os = "linux")]
impl From<&crate::protocol::PlaybackState> for StateKey {
    fn from(s: &crate::protocol::PlaybackState) -> Self {
        use mpris_server::PlaybackStatus;
        let status = match s.playback.as_deref().map(str::to_ascii_lowercase).as_deref() {
            Some("playing") => PlaybackStatus::Playing,
            Some("paused") => PlaybackStatus::Paused,
            _ => PlaybackStatus::Stopped,
        };
        let clean = |v: &Option<String>| v.as_ref().map(|x| x.trim().to_string()).filter(|x| !x.is_empty());
        Self {
            status,
            title: clean(&s.title),
            artist: clean(&s.artist),
            album: clean(&s.album),
        }
    }
}

#[cfg(target_os = "linux")]
async fn apply_state(player: &mpris_server::Player, key: &StateKey) -> Result<()> {
    use mpris_server::Metadata;
    let mut md = Metadata::builder();
    if let Some(t) = &key.title {
        md = md.title(t.clone());
    }
    if let Some(a) = &key.artist {
        md = md.artist([a.clone()]);
    }
    if let Some(a) = &key.album {
        md = md.album(a.clone());
    }
    player.set_metadata(md.build()).await?;
    player.set_playback_status(key.status).await?;
    Ok(())
}

#[cfg(not(target_os = "linux"))]
pub async fn run_mpris_client(
    _connect: String,
    _token: Option<String>,
    _name: String,
    _poll_secs: u64,
) -> Result<()> {
    Err(anyhow::anyhow!("mpris-client is only implemented on Linux"))
}

fn sanitize_bus_part(input: &str) -> String {
    let mut s = String::new();
    for ch in input.chars() {
        if ch.is_ascii_alphanumeric() || ch == '_' {
            s.push(ch);
        } else {
            s.push('_');
        }
    }
    if s.is_empty() || s.chars().next().unwrap().is_ascii_digit() {
        s.insert(0, '_');
    }
    s
}

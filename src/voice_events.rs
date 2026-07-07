use std::{
    sync::{
        Arc,
        mpsc::{self, RecvTimeoutError},
    },
    time::Duration,
};

use dashmap::DashMap;
use poise::serenity_prelude::async_trait;
use songbird::{
    Event, EventContext, EventHandler as VoiceEventHandler,
    model::{
        id::UserId,
        payload::{ClientDisconnect, Speaking},
    },
};

#[derive(Clone)]
pub struct Receiver {
    inner: Arc<InnerReceiver>,
}

struct InnerReceiver {
    voice_workers: DashMap<u32, mpsc::Sender<Vec<i16>>>,
}

impl Receiver {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(InnerReceiver {
                voice_workers: DashMap::new(),
            }),
        }
    }

    fn create_voice_worker(&self, ssrc: u32, user_id: UserId) {
        let (tx, rx) = mpsc::channel::<Vec<i16>>();

        tokio::spawn(async move {
            let mut buffer = Vec::new();

            loop {
                match rx.recv_timeout(Duration::from_secs(2)) {
                    Ok(chunk) => {
                        buffer.extend_from_slice(&chunk);
                    }
                    Err(RecvTimeoutError::Timeout) => {
                        if !buffer.is_empty() {
                            process_voice_clip(&buffer).await;
                            buffer.clear();
                        }
                    }
                    Err(RecvTimeoutError::Disconnected) => break,
                }
            }
        });

        self.inner.voice_workers.insert(ssrc, tx);
    }
}

async fn process_voice_clip(voice_data: &[i16]) {
    let _ = voice_data;
    // TODO: process the buffered voice clip.
}

#[async_trait]
impl VoiceEventHandler for Receiver {
    async fn act(&self, ctx: &EventContext<'_>) -> Option<Event> {
        use EventContext as Ctx;
        match ctx {
            Ctx::SpeakingStateUpdate(Speaking { ssrc, user_id, .. }) => {
                if let Some(user) = user_id {
                    self.create_voice_worker(*ssrc, *user);
                }
            }
            Ctx::VoiceTick(tick) => {
                for (ssrc, data) in &tick.speaking {
                    let Some(worker) = self.inner.voice_workers.get(ssrc) else {
                        continue;
                    };

                    if let Some(decoded_voice) = &data.decoded_voice {
                        let _ = worker.send(decoded_voice.to_owned());
                    }
                }
            }
            Ctx::ClientDisconnect(ClientDisconnect { user_id, .. }) => {
                // You can implement your own logic here to handle a user who has left the
                // voice channel e.g., finalise processing of statistics etc.
                // You will typically need to map the User ID to their SSRC; observed when
                // first speaking.

                println!("Client disconnected: user {:?}", user_id);
            }
            _ => {
                // We won't be registering this struct for any more event classes.
                unimplemented!()
            }
        }

        None
    }
}

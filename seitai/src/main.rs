use std::{collections::HashMap, env, sync::Arc};

use serenity::{client::Client, futures::lock::Mutex, model::gateway::GatewayIntents, prelude::TypeMapKey};
use songbird::{
    driver::Bitrate,
    input::{cached::Compressed, File},
    SerenityInit,
};

mod commands;
mod event_handler;
mod utils;
mod voicevox;

struct SoundStore;

impl TypeMapKey for SoundStore {
    type Value = Arc<Mutex<HashMap<String, Compressed>>>;
}

#[tokio::main]
async fn main() {
    let token = env::var("DISCORD_TOKEN").expect("`DISCORD_TOKEN` is not set.");

    let intents = GatewayIntents::non_privileged() | GatewayIntents::MESSAGE_CONTENT;
    let mut client = Client::builder(token, intents)
        .event_handler(event_handler::Handler)
        .register_songbird()
        .await
        .expect("Error creating client");
    {
        let mut data = client.data.write().await;

        let mut audio_map = HashMap::new();
        let url = Compressed::new(File::new("resources/url.wav").into(), Bitrate::BitsPerSecond(128_000))
            .await
            .expect("These parameters are well-defined.");
        let _ = url.raw.spawn_loader();

        audio_map.insert("URL".into(), url);

        data.insert::<SoundStore>(Arc::new(Mutex::new(audio_map)));
    }

    if let Err(why) = client.start().await {
        println!("Client error: {why:?}");
    }
}

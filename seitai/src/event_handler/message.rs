use std::{borrow::Cow, error::Error, ops::DerefMut, sync::Arc, time::Duration};

use anyhow::{Context as _, Result, bail};
use futures::{StreamExt, TryFutureExt, future, stream};
use hashbrown::{HashMap, HashSet};
use http_body_util::{BodyExt, Empty};
use hyper::{
    Request,
    StatusCode,
    body::{Body, Buf, Bytes},
};
use hyper_util::rt::TokioIo;
use ordered_float::NotNan;
use regex_lite::{Captures, Regex};
use serde::{Deserialize, de::DeserializeOwned};
use serenity::all::{Channel::Guild, ChannelId, Context, GuildId, Message};
use songbird::{Call, input::Input};
use soundboard::sound::SoundId;
use tokio::{net::TcpStream, sync::Mutex};
use url::Url;
use voicevox::dictionary::response::GetUserDictResult;

use crate::{
    audio::{Audio, AudioRepository, cache::PredefinedUtterance},
    character_converter,
    event_handler::Handler,
    regex,
    songbird_manager::SongbirdManager,
    speaker::Speaker,
    utils,
};

struct MessageHandler<'a, Repository> {
    event_handler: &'a Handler<Repository>,
    context: &'a Context,
    message: &'a Message,
    songbird_manager: SongbirdManager<'a>,
}

#[derive(Deserialize)]
pub(crate) struct Arpabet {
    word: String,
    pronunciation: Vec<String>,
}

#[derive(Deserialize)]
pub(crate) struct Katakana {
    pronunciation: String,
}

enum Replacement {
    General(&'static Regex, &'static str),
    Katakana(&'static Regex),
}

impl<'a, Repository> MessageHandler<'a, Repository>
where
    Repository: AudioRepository<Input = Input> + Send + Sync,
{
    fn new(event_handler: &'a Handler<Repository>, context: &'a Context, message: &'a Message) -> Self {
        Self {
            event_handler,
            context,
            message,
            songbird_manager: SongbirdManager::new(context),
        }
    }

    async fn should_skip(&self) -> Result<bool> {
        // Skip when the message is authored by bot
        if self.message.author.bot {
            return Ok(true);
        }

        // Skip when the message is outside of the guild
        let Some(guild_id) = self.message.guild_id else {
            return Ok(true);
        };

        let call = self.songbird_manager.call(guild_id).await?;

        // Skip when connection doesn't exist
        if call.lock().await.current_connection().is_none() {
            return Ok(true);
        }

        // Skip when bot isn't connnected to channel
        let Some(channel_id_bot_at) = call
            .lock()
            .await
            .current_channel()
            .map(|v| ChannelId::from(v.0))
        else {
            return Ok(true);
        };

        let is_voice_channel_bot_at = self
            .event_handler
            .connections
            .lock()
            .await
            .get(&guild_id)
            .is_some_and(|channel_id| &self.message.channel_id == channel_id);
        let is_text_channel_binded_to_bot = self.message.channel_id == channel_id_bot_at;

        // Skip when bot isn't connected to the channel where message was posted
        if !is_voice_channel_bot_at && !is_text_channel_binded_to_bot {
            return Ok(true);
        }

        let channel_bot_at = channel_id_bot_at
            .to_channel(&self.context.http)
            .await
            .with_context(|| format!("failed to get channel: {channel_id_bot_at:?}"))?;

        // Skip when bot isn't connected to the voice channel of guild
        let Guild(channel_bot_at) = channel_bot_at else {
            return Ok(true);
        };

        // Skip when message author isn't voice channel member
        if !channel_bot_at
            .members(&self.context.cache)
            .with_context(|| format!("failed to get members in channel: {channel_id_bot_at:?}"))?
            .into_iter()
            .map(|member| member.user)
            .any(|user| self.message.author == user)
        {
            return Ok(true);
        }

        Ok(false)
    }

    async fn handle_sticker(&self) -> Result<()> {
        let Some(guild_id) = self.message.guild_id else {
            return Ok(());
        };

        let call = self.songbird_manager.call(guild_id).await?;

        let Some(channel_id_bot_at) = call
            .lock()
            .await
            .current_channel()
            .map(|v| ChannelId::from(v.0))
        else {
            return Ok(());
        };

        let sticker_ids = self.message.sticker_items.clone().into_iter().map(|v| v.id.get());
        let soundstickers = database::soundsticker::fetch_by_ids(&self.event_handler.database, sticker_ids.clone())
            .await
            .with_context(|| format!("failed to fetch soundstickers by ids {:?}", sticker_ids.collect::<Vec<_>>()))?;

        for soundsticker in soundstickers {
            let sound_id = SoundId::new(soundsticker.sound_id);
            let sound_guild_id = soundsticker.sound_guild_id.map(GuildId::new).or(Some(guild_id));

            let mut last_sent = self.event_handler.time_keeper.lock().await;
            // guild_id in params of last_sent is where bot sent sound, not where sound is registered.
            let key = (guild_id, sound_id);
            if !last_sent.elapsed(&key, Duration::from_secs(10)) {
                last_sent.record(key);
                continue;
            }

            sound_id
                .send(&self.context.http, channel_id_bot_at, sound_guild_id)
                .await
                .with_context(|| format!("failed to send soundboard sound {sound_id:?}"))?;
            last_sent.record(key);
        }

        Ok(())
    }

    async fn handle_text(&self, call: Arc<Mutex<Call>>, speaker: String, speed: f32) -> Result<()> {
        let dictionary = {
            let voicevox = utils::get_voicevox(self.context)
                .await
                .context("failed to get voicevox client for /dictionary command")
                .unwrap();
            let voicevox = voicevox.lock().await;
            voicevox.dictionary.clone()
        };
        let dictionary_words = dictionary
            .list()
            .await
            .map(|GetUserDictResult::Ok(list)| {
                list.values()
                    .map(|item| character_converter::to_half_width(&item.surface).into_owned())
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();

        let message = replace_message(
            self.context,
            self.message,
            &self.event_handler.kanatrans_host,
            self.event_handler.kanatrans_port,
            &dictionary_words,
        )
            .await;
        let lines = message
            .lines()
            .filter_map(|v| {
                let text = v.trim();
                (!text.is_empty()).then_some(text)
            });

        for text in lines {
            let audio = Audio {
                text: text.to_string(),
                speaker: speaker.clone(),
                speed: NotNan::new(speed).or(NotNan::new(Speaker::default_speed())).unwrap(),
            };

            enqueue(call.lock().await, audio, &self.event_handler.audio_repository).await?;
        }

        Ok(())
    }

    async fn handle_attachment(&self, call: Arc<Mutex<Call>>, speaker: String, speed: f32) -> Result<()> {
        let audio = Audio {
            text: PredefinedUtterance::Attachment.as_ref().to_string(),
            speaker,
            speed: NotNan::new(speed).or(NotNan::new(Speaker::default_speed())).unwrap(),
        };

        enqueue(call.lock().await, audio, &self.event_handler.audio_repository).await?;

        Ok(())
    }

    async fn handle(&self) -> Result<()> {
        if self.should_skip().await? {
            return Ok(());
        };

        let Some(guild_id) = self.message.guild_id else {
            return Ok(());
        };

        let call = self.songbird_manager.call(guild_id).await?;

        if !self.message.sticker_items.is_empty() {
            self.handle_sticker().await?;
        }

        let ids: Vec<i64> = vec![self.message.author.id.into()];
        let speaker = database::user::fetch_by_ids(&self.event_handler.database, &ids)
            .await
            .with_context(|| format!("failed to fetch users by ids: {ids:?}"))?
            .first()
            .unwrap_or(&database::user::User::default())
            .speaker_id
            .to_string();

        let default = database::user::UserSpeaker::default();
        let speed = database::user::fetch_with_speaker_by_ids(&self.event_handler.database, &ids)
            .await
            .context("failed to fetch speakers")?
            .first()
            .unwrap_or(&default)
            .speed
            .or(default.speed)
            .unwrap_or(1.2);

        if !self.message.content.is_empty() {
            self.handle_text(call.clone(), speaker.clone(), speed).await?;
        }

        if !self.message.attachments.is_empty() {
            self.handle_attachment(call.clone(), speaker.clone(), speed).await?;
        }

        Ok(())
    }
}

pub(crate) async fn handle<Repository>(event_handler: &Handler<Repository>, context: Context, message: Message) -> Result<()>
where
    Repository: AudioRepository<Input = Input> + Send + Sync,
{
    let handler = MessageHandler::new(event_handler, &context, &message);
    handler.handle().await
}

async fn enqueue(mut call: impl DerefMut<Target = Call>, audio: Audio, audio_repository: &impl AudioRepository<Input = Input>) -> Result<()> {
    let input = audio_repository
        .get(audio)
        .await
        .context("failed to get audio source")?;

    call.enqueue_input(input).await;

    Ok(())
}

async fn replace_message<'a>(
    context: &Context,
    message: &'a Message,
    kanatrans_host: &str,
    kanatrans_port: u16,
    dictionary_words: &[String],
) -> Cow<'a, str> {
    let Some(guild_id) = message.guild_id else {
        return Cow::Borrowed(&message.content);
    };

    let replacements = [
        Replacement::General(&regex::CODE, "\nコード省略\n"),
        Replacement::General(&regex::URL, "\nURL\n"),
        Replacement::General(&regex::WW, "$1ワラワラ$2"),
        Replacement::General(&regex::W, "$1ワラ$2"),
        Replacement::General(&regex::IDEOGRAPHIC_FULL_STOP, "。\n"),
        Replacement::General(&regex::EMOJI, ":$1:"),
        Replacement::Katakana(&regex::WORD),
    ];

    let text = utils::normalize(context, &guild_id, &message.mentions, &message.content);
    stream::iter(replacements.into_iter())
        .fold(text, |accumulator, replacement| async move {
            match replacement {
                Replacement::General(regex, replacer) => match regex.replace_all(&accumulator, replacer) {
                    Cow::Borrowed(borrowed) if borrowed.len() == accumulator.len() => accumulator,
                    Cow::Borrowed(borrowed) => Cow::Owned(borrowed.to_owned()),
                    Cow::Owned(owned) => Cow::Owned(owned),
                },
                Replacement::Katakana(regex) => {
                    let accumulator = &accumulator;

                    let conversion_map = stream::iter(
                        regex
                            .find_iter(accumulator)
                            .map(|word| word.as_str())
                            .collect::<HashSet<_>>(),
                    )
                    .map(|word| async move {
                        if word.chars().all(char::is_uppercase)
                            || dictionary_words.iter().any(|dictionary_word| dictionary_word == word)
                        {
                            return None;
                        }

                        match get_arpabet(kanatrans_host, kanatrans_port, word)
                            .and_then(async |arpabet| {
                                get_katakana(
                                    kanatrans_host,
                                    kanatrans_port,
                                    Some(&arpabet.word),
                                    &arpabet.pronunciation,
                                )
                                .await
                            })
                            .await
                        {
                            Ok(katakana) => Some((word, katakana.pronunciation)),
                            Err(err) => {
                                tracing::error!("failed to get katakana\nError: {err:?}");
                                None
                            },
                        }
                    })
                    .collect::<Vec<_>>()
                    .await;

                    let conversion_map = future::join_all(conversion_map)
                        .await
                        .into_iter()
                        .flatten()
                        .collect::<HashMap<_, _>>();

                    if conversion_map.is_empty() {
                        return accumulator.clone();
                    }

                    let replaced = regex.replace_all(accumulator, |captures: &Captures| {
                        let word = &captures[0];
                        match conversion_map.get(word) {
                            Some(pronunciation) => pronunciation.to_owned(),
                            None => word.to_owned(),
                        }
                    });

                    match replaced {
                        Cow::Borrowed(borrowed) if borrowed.len() == accumulator.len() => accumulator.clone(),
                        Cow::Borrowed(borrowed) => Cow::Owned(borrowed.to_owned()),
                        Cow::Owned(owned) => Cow::Owned(owned),
                    }
                },
            }
        })
        .await
}

async fn request<RequestBody, Response>(url: Url, request: Request<RequestBody>) -> Result<(StatusCode, Response)>
where
    RequestBody: Body + Send + Unpin + 'static,
    RequestBody::Data: Send,
    RequestBody::Error: Into<Box<dyn Error + Send + Sync>>,
    Response: DeserializeOwned,
{
    let address = url.socket_addrs(|| None)?;
    let stream = TcpStream::connect(&*address).await?;
    let io = TokioIo::new(stream);
    let (mut sender, connection) = hyper::client::conn::http1::handshake(io).await?;

    tokio::task::spawn(async move {
        if let Err(error) = connection.await {
            tracing::error!("connection failed\nError: {error:?}");
        }
    });

    let response = sender.send_request(request).await?;
    let status = response.status();
    let body = response.collect().await?.aggregate();
    let json = serde_json::from_reader(body.reader())?;

    Ok((status, json))
}

async fn get_arpabet(host: &str, port: u16, word: &str) -> Result<Arpabet> {
    let url = Url::parse(&format!("http://{host}:{port}/arpabet/{word}"))?;
    let req = Request::get(url.path())
        .header(hyper::header::HOST, url.authority())
        .body(Empty::<Bytes>::new())
        .with_context(|| format!("failed to request with GET {url}"))?;
    let (status, arpabet) = request(url, req).await?;

    match status {
        StatusCode::OK => Ok(arpabet),
        StatusCode::UNPROCESSABLE_ENTITY => bail!("cannot convert {word} to ARPAbet"),
        code => bail!("received unexpected {code} from GET /arpabet/{word}"),
    }
}

async fn get_katakana(host: &str, port: u16, word: Option<&str>, pronunciation: &[String]) -> Result<Katakana> {
    let pronunciation = pronunciation.join(" ");
    let mut params = HashMap::new();
    params.insert("pronunciation", pronunciation.as_str());
    if let Some(word) = word {
        params.insert("word", word);
    }
    let url = Url::parse_with_params(&format!("http://{host}:{port}/katakana"), params)?;
    let uri = format!("{}?{}", url.path(), url.query().unwrap_or_default());
    let req = Request::get(uri)
        .header(hyper::header::HOST, url.authority())
        .header(hyper::header::CONTENT_TYPE, "application/json")
        .body(Empty::<Bytes>::new())
        .with_context(|| format!("failed to request with GET {url}"))?;
    let (status, katakana) = request(url.clone(), req).await?;

    match status {
        StatusCode::OK => Ok(katakana),
        code => bail!(
            "received unexpected {code} from GET /katakana with {}",
            url.query().unwrap_or_default()
        ),
    }
}

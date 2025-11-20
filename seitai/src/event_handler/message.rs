use std::{borrow::Cow, error::Error, time::Duration};

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
use serenity::all::{ChannelId, Context, GuildId, Message};
use songbird::input::Input;
use soundboard::sound::SoundId;
use tokio::net::TcpStream;
use url::Url;
use voicevox::dictionary::response::GetUserDictResult;

use crate::{
    audio::{Audio, AudioRepository, cache::PredefinedUtterance},
    character_converter,
    event_handler::Handler,
    regex,
    speaker::Speaker,
    utils,
};

struct MessageHandler<'a, Repository> {
    event_handler: &'a Handler<Repository>,
    context: Context,
    message: Message,
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
    fn new(event_handler: &'a Handler<Repository>, context: Context, message: Message) -> Self {
        Self { event_handler, context, message }
    }

    async fn handle(&self) {
        if self.message.author.bot {
            return;
        }

        let Some(guild_id) = self.message.guild_id else {
            return;
        };

        let manager = match utils::get_manager(&self.context).await {
            Ok(manager) => manager,
            Err(error) => {
                tracing::error!("{error:?}");
                return;
            },
        };
        let call = manager.get_or_insert(guild_id);
        let mut call = call.lock().await;

        let (Some(_), Some(channel_id_bot_at)) = (call.current_connection(), call.current_channel()) else {
            return;
        };
        let channel_id_bot_at = ChannelId::from(channel_id_bot_at.0);

        let is_voice_channel_bot_at = {
            let connections = self.event_handler.connections.lock().await;
            connections
                .get(&guild_id)
                .is_some_and(|channel_id| &self.message.channel_id == channel_id)
        };
        let is_text_channel_binded_to_bot = self.message.channel_id == channel_id_bot_at;

        if !is_voice_channel_bot_at && !is_text_channel_binded_to_bot {
            return;
        }

        let channel_bot_at = match channel_id_bot_at.to_channel(&self.context.http).await {
            Ok(channel_bot_at) => channel_bot_at,
            Err(error) => {
                tracing::error!("failed to get channel: {channel_id_bot_at:?}\nError: {error:?}");
                return;
            },
        };

        let serenity::all::Channel::Guild(channel_bot_at) = channel_bot_at else {
            return;
        };

        let members = match channel_bot_at.members(&self.context.cache) {
            Ok(members) => members,
            Err(error) => {
                tracing::error!("failed to get members in channel: {channel_bot_at:?}\nError: {error:?}");
                return;
            },
        };
        if !members
            .into_iter()
            .map(|member| member.user)
            .any(|user| self.message.author == user)
        {
            return;
        }

        if !self.message.sticker_items.is_empty() {
            let sticker_ids = self.message.sticker_items.clone().into_iter().map(|v| v.id.get());
            let soundstickers = match database::soundsticker::fetch_by_ids(&self.event_handler.database, sticker_ids.clone()).await {
                Ok(soundstickers) => soundstickers,
                Err(err) => {
                    tracing::error!("failed to fetch soundstickers by ids: {:?}\nError: {err:?}", sticker_ids.collect::<Vec<_>>());
                    return;
                },
            };

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

                match sound_id.send(&self.context.http, channel_id_bot_at, sound_guild_id).await {
                    Ok(_) => last_sent.record(key),
                    Err(err) => {
                        tracing::error!("failed to send soundboard sound {sound_id:?}\nError: {err:?}");
                        continue;
                    },
                };
            }

            return;
        }

        let ids: Vec<i64> = vec![self.message.author.id.into()];
        let speaker = match database::user::fetch_by_ids(&self.event_handler.database, &ids).await {
            Ok(users) => users
                .first()
                .unwrap_or(&database::user::User::default())
                .speaker_id
                .to_string(),
            Err(error) => {
                tracing::error!("failed to fetch users by ids: {ids:?}\nError: {error:?}");
                return;
            },
        };

        let default = database::user::UserSpeaker::default();
        let speed =
        match database::user::fetch_with_speaker_by_ids(&self.event_handler.database, &[self.message.author.id.into()]).await {
            Ok(speakers) => speakers
                .first()
                .unwrap_or(&default)
                .speed
                .or(default.speed)
                .unwrap_or(1.2),
            Err(error) => {
                tracing::error!("failed to fetch speakers\nError: {error:?}");
                return;
            },
        };

        {
            let dictionary = {
                let voicevox = utils::get_voicevox(&self.context)
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

            for text in replace_message(
                &self.context,
                &self.message,
                &self.event_handler.kanatrans_host,
                self.event_handler.kanatrans_port,
                &dictionary_words,
            )
                .await
                .split('\n')
            {
                let text = text.trim();
                if text.is_empty() {
                    continue;
                }

                let audio = Audio {
                    text: text.to_string(),
                    speaker: speaker.clone(),
                    speed: NotNan::new(speed).or(NotNan::new(Speaker::default_speed())).unwrap(),
                };
                match self.event_handler.audio_repository.get(audio).await {
                    Ok(input) => {
                        call.enqueue_input(input).await;
                    },
                    Err(error) => {
                        tracing::error!("failed to get audio source\nError: {error:?}");
                    },
                };
            }

            if !self.message.attachments.is_empty() {
                let audio = Audio {
                    text: PredefinedUtterance::Attachment.as_ref().to_string(),
                    speaker: speaker.clone(),
                    speed: NotNan::new(speed).or(NotNan::new(Speaker::default_speed())).unwrap(),
                };
                match self.event_handler.audio_repository.get(audio).await {
                    Ok(input) => {
                        call.enqueue_input(input).await;
                    },
                    Err(error) => {
                        tracing::error!("failed to get audio source\nError: {error:?}");
                    },
                };
            }
        }
    }
}

pub(crate) async fn handle<Repository>(event_handler: &Handler<Repository>, context: Context, message: Message)
where
    Repository: AudioRepository<Input = Input> + Send + Sync,
{
    let handler = MessageHandler::new(event_handler, context, message);
    handler.handle().await;
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

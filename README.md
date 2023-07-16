# Seitai

## 必要なもの

### イメージ

https://github.com/VOICEVOX/voicevox_engine

```sh
docker pull voicevox/voicevox_engine:cpu-ubuntu20.04-latest
```

### botの権限

- SCOPES: `bot`
- BOT PERMISSIONS
  - `Send Message` (TEXT PERMISSIONS)
  - `Connect` (VOICE PERMISSIONS)
  - `Speak` (VOICE PERMISSIONS)
- Privileged Gateway Intents: `MESSAGE CONTENT INTENT`

## 環境変数
- `DISCORD_TOKEN`: Discordのbotのトークン
- `VOICEVOX_HOST`: VOICEVOX ENGINEのコンテナーのホスト名

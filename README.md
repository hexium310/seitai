# Seitai

## 必要なもの

### イメージ

https://github.com/VOICEVOX/voicevox_engine

```sh
docker pull voicevox/voicevox_engine:cpu-ubuntu20.04-latest
```

### bot の権限

- SCOPES: `bot`
- BOT PERMISSIONS
  - `Send Message` (TEXT PERMISSIONS)
  - `Connect` (VOICE PERMISSIONS)
  - `Speak` (VOICE PERMISSIONS)
- Privileged Gateway Intents: `MESSAGE CONTENT INTENT`

## 環境変数

- `DISCORD_TOKEN`: Discord の bot のトークン
- `VOICEVOX_HOST`: VOICEVOX ENGINE のコンテナーのホスト名

[.envrc.sample](.envrc.sample) も確認してください。

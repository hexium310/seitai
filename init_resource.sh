set -e
set -o pipefail

generate_audio() {
    local text="$1"
    local file="$2"

    [[ -d ./resources ]] || mkdir ./resources

    if [[ ! -e "./resources/$file" ]]; then
        local json
        json=$(curl --fail --silent --request POST --get --data-urlencode "text=$text" "$VOICEVOX_HOST:50021/audio_query?speaker=1" | sed -e 's/"speedScale":1.0/"speedScale":1.2/')
        curl --fail --silent --request POST --header 'Content-Type: application/json' --data "$json" "$VOICEVOX_HOST:50021/synthesis?speaker=1" > "./resources/$file"
        voicevox_is_used=1
    fi
}

generate_audio 'コード省略' code.wav
generate_audio '接続しました' connected.wav
generate_audio URL url.wav
generate_audio '添付ファイル' attachment.wav
generate_audio 'を登録しました' registered.wav

echo $(ls ./resources) are available

if [[ -n "$KUBERNETES_SERVICE_HOST" ]] && [[ "${voicevox_is_used:+defined}" ]]; then
    kubectl rollout restart statefulset voicevox
fi

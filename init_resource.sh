set -e

generate_audio() {
    local text="$1"
    local file="$2"

    [[ -d ./resources ]] || mkdir ./resources

    if [[ ! -e "./resources/$file" ]]; then
        local json
        json=$(curl --silent --request POST --get --data-urlencode "text=$text" "$VOICEVOX_HOST:50021/audio_query?speaker=1")
        curl --silent --request POST --header 'Content-Type: application/json' --data "$json" "$VOICEVOX_HOST:50021/synthesis?speaker=1" > ./resources/url.wav
        voicevox_is_used=1
    fi
}

generate_audio URL url.wav
echo $(ls ./resources) are available

if [[ -n "$KUBERNETES_SERVICE_HOST" ]] && [[ "${voicevox_is_used:+defined}" ]]; then
    kubectl rollout restart statefulset voicevox
fi

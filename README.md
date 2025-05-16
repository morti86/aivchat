# aivchat
The new and better AI voice chat app written using Iced combining the power of OpanAI Whisper and Elevenlabs to (at least try to) let you chat with the AI.

It can also be used as a voice transcription app so you can talk to the microphone to create texts. It can record output too, so you can transcribe the dialogues of YT videos or similar things.

The app requires Vulkan since the Whisper library uses it for GPU acceleration. Need to download the ggml model file from [here]: https://huggingface.co/ggerganov/whisper.cpp/tree/main
Tested with v3 large turbo model

https://huggingface.co/ggerganov/whisper.cpp/blob/main/ggml-large-v3-turbo-q5_0.bin

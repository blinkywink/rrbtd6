# RustTD6 patches to quad-snd 0.2.8

Upstream: https://github.com/not-fl3/quad-snd
Author: Fedor Logachev. Upstream declares MIT OR Apache-2.0 licensing.
The upstream README and Cargo metadata are retained.

- Enable audrey's lossless FLAC decoder alongside the existing formats.
- Expose pure `decode_samples` on native platforms, allowing worker decoding.
- Expose `Sound::from_samples` on native backends. Prepared PCM enters the
  existing mixer queue, without a second decode.
- Prepare the shared PCM allocation on the worker; the mixer queue transfers
  its Arc directly instead of copying a large Vec into an Arc on the audio thread.
- Propagate decoder errors rather than panicking on malformed files.

Mixing, mono duplication, resampling, playback and volume math are unchanged.

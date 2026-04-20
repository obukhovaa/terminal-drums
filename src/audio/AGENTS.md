# AGENTS.md — Audio Module

- `AudioEngine::manager_and_metronome_track()` exists to work around the borrow checker — you can't borrow `manager` mutably and `metronome_track` immutably from the same struct. Use this helper instead of separate accessors.
- Kit samples are `StaticSoundData` (preloaded in memory). Backtracks are `StreamingSoundData` (streamed from disk). Don't mix these up — StaticSoundData for short hits, StreamingSoundData for long audio.
- Missing kit samples are non-fatal. `DrumKit::load` silently skips missing WAV files. The engine just won't play audio for that piece.
- Mute is implemented via track volume (not by skipping play calls). This means un-muting during playback resumes audio seamlessly without needing to restart sounds.

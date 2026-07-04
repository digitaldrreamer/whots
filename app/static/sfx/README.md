# Sound assets

Real audio files served at `/sfx/<name>` (SvelteKit serves `static/` at the web root).
Everything else in the game is synthesized in `src/lib/ui/sound.ts`; only these
one-shot recordings are loaded from disk.

## Expected files

| file | used for | spec |
|------|----------|------|
| `tee-laugh.mp3` | Tee-Noble's evil laugh — plays when he takes his seat and when he wins | mono/stereo MP3, **1.5–3.5 s**, evil/maniacal villain laugh, normalized to ~−14 LUFS, trimmed (no leading silence) |

Missing files are handled gracefully — the sound simply doesn't play, nothing breaks.

Good CC0 sources: freesound.org (License: Creative Commons 0), pixabay.com/sound-effects.
Search terms: "evil laugh", "villain laugh", "maniacal laughter", "sinister laugh".

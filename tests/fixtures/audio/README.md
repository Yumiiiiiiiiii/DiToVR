# Audio regression fixtures

These MP3 files contain an original, generated 660 Hz sine wave for 0.2 seconds
at 22,050 Hz. They contain no user recordings or third-party audio.

Generated with FFmpeg's `sine` filter and `libmp3lame`, with one and two channels:

```text
ffmpeg -f lavfi -i sine=frequency=660:duration=0.2:sample_rate=22050 -ac 1 -codec:a libmp3lame -b:a 64k -map_metadata -1 sine-1ch.mp3
ffmpeg -f lavfi -i sine=frequency=660:duration=0.2:sample_rate=22050 -ac 2 -codec:a libmp3lame -b:a 64k -map_metadata -1 sine-2ch.mp3
```

FFmpeg is only used to generate these test fixtures. DiToVR does not invoke or
require FFmpeg at runtime.

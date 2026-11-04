//! Prepare user-selected sounds once, outside the notification and UI threads.
use sha2::{Digest, Sha256};
use std::{
    fs::{self, File},
    io::{Cursor, Read},
    num::{NonZeroU32, NonZeroU8},
    path::Path,
    time::{Duration, Instant},
};
use symphonia::core::{audio::SampleBuffer, errors::Error, io::MediaSourceStream, probe::Hint};
use vorbis_rs::VorbisEncoderBuilder;

const MAX_INPUT_BYTES: u64 = 16 * 1024 * 1024;
const MAX_SECONDS: u64 = 120;
const CACHE_PREFIX: &str = "ditovr-audio-v1-";
const MAX_CACHED_SOUNDS: usize = 8;
const INVALID_AUDIO: &str =
    "音声を読み込めません。MP3・WAV・Ogg Vorbis のファイルを選んでください。";

pub fn validate_sound_path(sound: &str) -> Result<(), &'static str> {
    if matches!(sound, "" | "default" | "error" | "warning") {
        return Ok(());
    }
    let path = Path::new(sound);
    if !path.is_absolute() {
        return Err("通知音のファイルを「参照」から選んでください。");
    }
    if !path.extension().is_some_and(|ext| {
        ["mp3", "wav", "ogg"]
            .iter()
            .any(|supported| ext.eq_ignore_ascii_case(supported))
    }) {
        return Err("通知音は MP3・WAV・Ogg Vorbis（.ogg）に対応しています。");
    }
    Ok(())
}

/// MP3/WAV become content-addressed Ogg files. Configuration keeps the original path.
/// This function is blocking: desktop callers use spawn_blocking before starting services.
pub fn prepare_sound(sound: &str, cache_directory: &Path) -> Result<String, &'static str> {
    validate_sound_path(sound)?;
    if matches!(sound, "" | "default" | "error" | "warning") {
        return Ok(sound.into());
    }
    let file = File::open(sound)
        .map_err(|_| "通知音のファイルを読み込めません。「参照」から選び直してください。")?;
    let mut bytes = Vec::new();
    file.take(MAX_INPUT_BYTES + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| INVALID_AUDIO)?;
    if bytes.len() as u64 > MAX_INPUT_BYTES {
        return Err("通知音は16 MiB以下のファイルを選んでください。");
    }
    if Path::new(sound)
        .extension()
        .is_some_and(|ext| ext.eq_ignore_ascii_case("ogg"))
    {
        // XSOverlay decodes Vorbis, not Opus. Verify the actual stream as well as the extension.
        validate_vorbis(&bytes)?;
        return Ok(sound.into());
    }
    let digest = Sha256::digest(&bytes);
    let destination = cache_directory.join(format!("{CACHE_PREFIX}{digest:x}.ogg"));
    if let Ok(mut cached) = File::open(&destination) {
        let mut cached_bytes = Vec::new();
        if cached
            .by_ref()
            .take(MAX_INPUT_BYTES + 1)
            .read_to_end(&mut cached_bytes)
            .is_ok()
            && cached_bytes.len() as u64 <= MAX_INPUT_BYTES
            && validate_vorbis(&cached_bytes).is_ok()
        {
            return Ok(destination.to_string_lossy().into());
        }
    }
    let ogg = transcode(bytes)?;
    fs::create_dir_all(cache_directory).map_err(|_| "通知音の変換先を作成できませんでした。")?;
    crate::config::write_generated_file(&destination, &ogg)
        .map_err(|_| "変換した通知音を保存できませんでした。保存先を確認してください。")?;
    prune_cache(cache_directory, &destination);
    Ok(destination.to_string_lossy().into())
}

fn validate_vorbis(bytes: &[u8]) -> Result<(), &'static str> {
    let started = Instant::now();
    let mut decoder =
        vorbis_rs::VorbisDecoder::new(Cursor::new(bytes)).map_err(|_| INVALID_AUDIO)?;
    let mut frames = 0u64;
    let max_frames = u64::from(decoder.sampling_frequency().get()) * MAX_SECONDS;
    while let Some(block) = decoder.decode_audio_block().map_err(|_| INVALID_AUDIO)? {
        frames += block.samples()[0].len() as u64;
        if frames > max_frames {
            return Err("通知音は120秒以下のファイルを選んでください。");
        }
        if started.elapsed() > Duration::from_secs(15) {
            return Err("通知音の読み込みが時間内に終わりませんでした。短い音声を選んでください。");
        }
    }
    if frames == 0 {
        return Err(INVALID_AUDIO);
    }
    Ok(())
}

fn transcode(bytes: Vec<u8>) -> Result<Vec<u8>, &'static str> {
    let started = Instant::now();
    let serial = i32::from_le_bytes(Sha256::digest(&bytes)[..4].try_into().unwrap());
    let source = MediaSourceStream::new(Box::new(Cursor::new(bytes)), Default::default());
    let probed = symphonia::default::get_probe()
        .format(
            &Hint::new(),
            source,
            &Default::default(),
            &Default::default(),
        )
        .map_err(|_| INVALID_AUDIO)?;
    let mut format = probed.format;
    let track = format.default_track().ok_or(INVALID_AUDIO)?;
    let track_id = track.id;
    let mut decoder = symphonia::default::get_codecs()
        .make(&track.codec_params, &Default::default())
        .map_err(|_| INVALID_AUDIO)?;
    let rate = track.codec_params.sample_rate.ok_or(INVALID_AUDIO)?;
    let channels = track.codec_params.channels.ok_or(INVALID_AUDIO)?.count();
    if !(8_000..=192_000).contains(&rate) || !(1..=2).contains(&channels) {
        return Err("通知音は8～192 kHzのモノラルまたはステレオ音声を選んでください。");
    }
    let mut encoded = Vec::new();
    let mut encoder = VorbisEncoderBuilder::new_with_serial(
        NonZeroU32::new(rate).unwrap(),
        NonZeroU8::new(channels as u8).unwrap(),
        &mut encoded,
        serial,
    )
    .build()
    .map_err(|_| "通知音をOgg Vorbisに変換できませんでした。")?;
    let mut total_frames = 0u64;
    loop {
        if started.elapsed() > Duration::from_secs(15) {
            return Err("通知音の変換が時間内に終わりませんでした。短い音声を選んでください。");
        }
        let packet = match format.next_packet() {
            Ok(packet) => packet,
            Err(Error::IoError(error)) if error.kind() == std::io::ErrorKind::UnexpectedEof => {
                break
            }
            Err(_) => return Err(INVALID_AUDIO),
        };
        if packet.track_id() != track_id {
            continue;
        }
        let decoded = decoder.decode(&packet).map_err(|_| INVALID_AUDIO)?;
        if decoded.spec().rate != rate
            || decoded.spec().channels.count() != channels
            || decoded.capacity() > 192_000
        {
            return Err(INVALID_AUDIO);
        }
        let mut samples = SampleBuffer::<f32>::new(decoded.capacity() as u64, *decoded.spec());
        samples.copy_planar_ref(decoded);
        let frames = samples.len() / channels;
        total_frames += frames as u64;
        if total_frames > u64::from(rate) * MAX_SECONDS {
            return Err("通知音は120秒以下のファイルを選んでください。");
        }
        if frames > 0 {
            let planes: Vec<&[f32]> = samples.samples().chunks_exact(frames).collect();
            encoder
                .encode_audio_block(planes)
                .map_err(|_| INVALID_AUDIO)?;
        }
    }
    if total_frames == 0 {
        return Err(INVALID_AUDIO);
    }
    encoder.finish().map_err(|_| INVALID_AUDIO)?;
    if encoded.len() as u64 > MAX_INPUT_BYTES {
        return Err("変換した通知音が大きすぎます。短い音声を選んでください。");
    }
    Ok(encoded)
}

fn prune_cache(directory: &Path, keep: &Path) {
    let Ok(entries) = fs::read_dir(directory) else {
        return;
    };
    let mut generated: Vec<_> = entries
        .filter_map(Result::ok)
        .filter(|entry| {
            let name = entry.file_name();
            let Some(name) = name.to_str() else {
                return false;
            };
            let Some(digest) = name
                .strip_prefix(CACHE_PREFIX)
                .and_then(|s| s.strip_suffix(".ogg"))
            else {
                return false;
            };
            digest.len() == 64
                && digest.bytes().all(|c| c.is_ascii_hexdigit())
                && entry.file_type().is_ok_and(|kind| kind.is_file())
        })
        .collect();
    generated.sort_by_key(|entry| entry.metadata().and_then(|m| m.modified()).ok());
    let excess = generated.len().saturating_sub(MAX_CACHED_SOUNDS);
    for entry in generated
        .into_iter()
        .filter(|entry| entry.path() != keep)
        .take(excess)
    {
        let _ = fs::remove_file(entry.path());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    struct Directory(std::path::PathBuf);
    impl Directory {
        fn new() -> Self {
            let path =
                std::env::temp_dir().join(format!("ditovr-audio-test-{}", uuid::Uuid::new_v4()));
            fs::create_dir(&path).unwrap();
            Self(path)
        }
    }
    impl Drop for Directory {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }
    fn inspect(path: &str) -> (u8, u32, usize, f32) {
        let mut decoder = vorbis_rs::VorbisDecoder::new(File::open(path).unwrap()).unwrap();
        let (channels, rate) = (decoder.channels().get(), decoder.sampling_frequency().get());
        let (mut frames, mut energy) = (0, 0.0);
        while let Some(block) = decoder.decode_audio_block().unwrap() {
            frames += block.samples()[0].len();
            energy += block
                .samples()
                .iter()
                .flat_map(|channel| channel.iter())
                .map(|sample| sample.abs())
                .sum::<f32>();
        }
        (channels, rate, frames, energy)
    }
    fn wav(channels: u16, frames: u32) -> Vec<u8> {
        let data_size = frames * u32::from(channels) * 2;
        let mut bytes = Vec::new();
        bytes.extend_from_slice(b"RIFF");
        bytes.extend_from_slice(&(36 + data_size).to_le_bytes());
        bytes.extend_from_slice(b"WAVEfmt ");
        bytes.extend_from_slice(&16u32.to_le_bytes());
        bytes.extend_from_slice(&1u16.to_le_bytes());
        bytes.extend_from_slice(&channels.to_le_bytes());
        bytes.extend_from_slice(&22_050u32.to_le_bytes());
        bytes.extend_from_slice(&(22_050u32 * u32::from(channels) * 2).to_le_bytes());
        bytes.extend_from_slice(&(channels * 2).to_le_bytes());
        bytes.extend_from_slice(&16u16.to_le_bytes());
        bytes.extend_from_slice(b"data");
        bytes.extend_from_slice(&data_size.to_le_bytes());
        for frame in 0..frames {
            let sample =
                ((frame as f32 * 660.0 * std::f32::consts::TAU / 22_050.0).sin() * 8_000.0) as i16;
            for _ in 0..channels {
                bytes.extend_from_slice(&sample.to_le_bytes());
            }
        }
        bytes
    }

    #[test]
    fn actual_mp3_files_decode_to_playable_vorbis_and_reuse_the_cache() {
        let directory = Directory::new();
        for (channels, bytes) in [
            (
                1,
                &include_bytes!("../tests/fixtures/audio/sine-1ch.mp3")[..],
            ),
            (
                2,
                &include_bytes!("../tests/fixtures/audio/sine-2ch.mp3")[..],
            ),
        ] {
            let source = directory.0.join(format!("通知音 #1 % {channels}.MP3"));
            fs::write(&source, bytes).unwrap();
            let original = source.to_str().unwrap();
            let output = prepare_sound(original, &directory.0.join("cache")).unwrap();
            let (decoded_channels, rate, frames, energy) = inspect(&output);
            assert_eq!(decoded_channels, channels);
            assert_eq!(rate, 22_050);
            assert!(
                (4_000..=8_000).contains(&frames),
                "decoded frames: {frames}"
            );
            assert!(energy > 1.0);
            assert_eq!(fs::read(&source).unwrap(), bytes);
            let modified = fs::metadata(&output).unwrap().modified().unwrap();
            assert_eq!(
                prepare_sound(original, &directory.0.join("cache")).unwrap(),
                output
            );
            assert_eq!(fs::metadata(&output).unwrap().modified().unwrap(), modified);
            fs::write(&output, b"OggS damaged cache").unwrap();
            assert_eq!(
                prepare_sound(original, &directory.0.join("cache")).unwrap(),
                output
            );
            assert!(inspect(&output).3 > 1.0);
        }
    }

    #[test]
    fn wav_changes_invalidate_cache_and_vorbis_sources_remain_unchanged() {
        let directory = Directory::new();
        let source = directory.0.join("sound.wav");
        let original = source.to_str().unwrap();
        fs::write(&source, wav(1, 2_205)).unwrap();
        let first = prepare_sound(original, &directory.0.join("cache")).unwrap();
        assert_eq!(inspect(&first).0, 1);
        fs::write(&source, wav(2, 4_410)).unwrap();
        let second = prepare_sound(original, &directory.0.join("cache")).unwrap();
        assert_ne!(first, second);
        let (_, rate, frames, energy) = inspect(&second);
        assert_eq!(rate, 22_050);
        assert_eq!(frames, 4_410);
        assert!(energy > 1.0);
        let bytes = fs::read(&second).unwrap();
        assert_eq!(
            prepare_sound(&second, &directory.0.join("cache")).unwrap(),
            second
        );
        assert_eq!(fs::read(second).unwrap(), bytes);
        let unrelated = directory.0.join("cache/user-recording.ogg");
        fs::write(&unrelated, b"preserve").unwrap();
        for index in 0..10 {
            fs::write(&source, wav(1, 1_000 + index)).unwrap();
            prepare_sound(original, &directory.0.join("cache")).unwrap();
        }
        assert_eq!(fs::read(&unrelated).unwrap(), b"preserve");
        assert_eq!(
            fs::read_dir(directory.0.join("cache")).unwrap().count(),
            MAX_CACHED_SOUNDS + 1
        );
    }

    #[test]
    fn invalid_empty_and_oversized_files_do_not_create_cached_output() {
        let directory = Directory::new();
        let cache = directory.0.join("cache");
        let source = directory.0.join("bad.mp3");
        for bytes in [b"".as_slice(), b"not audio", b"OggS"] {
            fs::write(&source, bytes).unwrap();
            assert!(prepare_sound(source.to_str().unwrap(), &cache).is_err());
            assert!(!cache.exists());
        }
        File::create(&source)
            .unwrap()
            .set_len(MAX_INPUT_BYTES + 1)
            .unwrap();
        assert!(prepare_sound(source.to_str().unwrap(), &cache)
            .unwrap_err()
            .contains("16 MiB"));
        assert!(!cache.exists());
        for built_in in ["", "default", "error", "warning"] {
            assert_eq!(prepare_sound(built_in, &cache).unwrap(), built_in);
        }
    }
}

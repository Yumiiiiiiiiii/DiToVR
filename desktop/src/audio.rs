use rodio::{Decoder, OutputStreamBuilder, Sink, Source, source::SineWave};
use std::{fs::File, io::BufReader, path::PathBuf, time::Duration};

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Sound {
    Silent,
    Chime,
    File,
}

pub fn play(sound: Sound, file: Option<PathBuf>, volume: f32) -> Result<(), &'static str> {
    if sound == Sound::Silent {
        return Err("通知音を選んでからテストしてください。");
    }
    if volume <= 0. {
        return Err("音量を1%以上にしてください。");
    }
    let mut stream = OutputStreamBuilder::open_default_stream()
        .map_err(|_| "再生デバイスを開けません。Windowsの出力先を確認してください。")?;
    stream.log_on_drop(false);
    let sink = Sink::connect_new(stream.mixer());
    sink.set_volume(volume.clamp(0., 100.) / 100.);
    if sound == Sound::File {
        let file = file.ok_or("音声ファイルを選んでください。")?;
        let input = File::open(file).map_err(|_| "ファイルを開けません。選び直してください。")?;
        let bytes = input
            .metadata()
            .map_err(|_| "ファイルを確認できません。")?
            .len();
        if bytes == 0 || bytes > 16 * 1024 * 1024 {
            return Err("空ではない16 MiB以下のファイルを選んでください。");
        }
        let decoder = Decoder::try_from(BufReader::new(input))
            .map_err(|_| "音声を読み込めません。MP3・WAV・Ogg Vorbisを選んでください。")?;
        sink.append(decoder.take_duration(Duration::from_secs(4)));
    } else {
        sink.append(
            SineWave::new(740.)
                .take_duration(Duration::from_millis(180))
                .amplify(0.18),
        );
    }
    sink.sleep_until_end();
    Ok(())
}

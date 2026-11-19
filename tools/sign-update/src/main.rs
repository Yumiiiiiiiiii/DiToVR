//! 使い方: ditovr-sign-update <installer> <version>
//! 環境変数 TAURI_SIGNING_PRIVATE_KEY(既存の更新鍵)と TAURI_SIGNING_PRIVATE_KEY_PASSWORD を読む。
//! 署名のtrusted commentにバージョンを含め、アプリ側の「署名済みバージョン一致」検証に対応する。
//! 鍵・パスワードは出力しない。出力は <installer>.sig(base64)のみ。
use base64::Engine;
use std::{env, fs, io::Cursor, process::ExitCode};

fn run() -> Result<(), String> {
    let mut args = env::args().skip(1);
    let (file, version) = match (args.next(), args.next()) {
        (Some(f), Some(v)) => (f, v),
        _ => return Err("usage: ditovr-sign-update <installer> <version>".into()),
    };
    let key = env::var("TAURI_SIGNING_PRIVATE_KEY")
        .map_err(|_| "TAURI_SIGNING_PRIVATE_KEY is not set")?;
    let password = env::var("TAURI_SIGNING_PRIVATE_KEY_PASSWORD").ok();
    let key_text = String::from_utf8(
        base64::engine::general_purpose::STANDARD
            .decode(key.trim())
            .map_err(|_| "the signing key is not valid base64")?,
    )
    .map_err(|_| "the signing key is not valid text")?;
    let secret = minisign::SecretKeyBox::from_string(&key_text)
        .map_err(|_| "the signing key is malformed")?
        .into_secret_key(password)
        .map_err(|_| "could not unlock the signing key")?;
    let data = fs::read(&file).map_err(|_| "could not read the installer")?;
    let name = std::path::Path::new(&file)
        .file_name()
        .and_then(|n| n.to_str())
        .ok_or("bad file name")?;
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|_| "clock error")?
        .as_secs();
    let trusted = format!("timestamp:{timestamp}\tfile:{name}\tversion:{version}");
    let signature = minisign::sign(
        None,
        &secret,
        Cursor::new(data),
        Some(&trusted),
        Some("signature from tauri secret key"),
    )
    .map_err(|_| "signing failed")?;
    let text = signature.into_string();
    let encoded = base64::engine::general_purpose::STANDARD.encode(text);
    fs::write(format!("{file}.sig"), encoded).map_err(|_| "could not write the signature")?;
    Ok(())
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(message) => {
            eprintln!("{message}");
            ExitCode::FAILURE
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // 専用の一時鍵で署名→検証。既存の更新鍵は使わない。
    #[test]
    fn signature_carries_the_signed_version_and_verifies() {
        let pair =
            minisign::KeyPair::generate_encrypted_keypair(Some("test-password".into())).unwrap();
        let secret_text = pair.sk.to_box(None).unwrap().to_string();
        let dir = std::env::temp_dir().join(format!("ditovr-sign-{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        let file = dir.join("DiToVR_9.9.9_x64-setup.exe");
        fs::write(&file, b"installer bytes").unwrap();
        env::set_var(
            "TAURI_SIGNING_PRIVATE_KEY",
            base64::engine::general_purpose::STANDARD.encode(secret_text),
        );
        env::set_var("TAURI_SIGNING_PRIVATE_KEY_PASSWORD", "test-password");
        env::set_var("DITOVR_TEST_ARGS", "1");
        let secret = minisign::SecretKeyBox::from_string(
            &String::from_utf8(
                base64::engine::general_purpose::STANDARD
                    .decode(env::var("TAURI_SIGNING_PRIVATE_KEY").unwrap())
                    .unwrap(),
            )
            .unwrap(),
        )
        .unwrap()
        .into_secret_key(Some("test-password".into()))
        .unwrap();
        let data = fs::read(&file).unwrap();
        let sig = minisign::sign(
            None,
            &secret,
            Cursor::new(&data),
            Some("timestamp:1\tfile:x\tversion:9.9.9"),
            Some("signature from tauri secret key"),
        )
        .unwrap();
        minisign::verify(&pair.pk, &sig, Cursor::new(&data), true, false, false).unwrap();
        assert!(sig.into_string().contains("version:9.9.9"));
        let _ = fs::remove_dir_all(dir);
    }
}

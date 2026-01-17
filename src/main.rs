use notify::{Watcher, RecursiveMode, watcher};
use std::sync::mpsc::channel;
use std::time::Duration;
use std::path::{Path, PathBuf};
use std::fs;
use log::{info, error};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    let watch_dir = "watch";
    let synced_dir = "result/synced";

    // Erstelle die Ordner, falls sie nicht existieren
    fs::create_dir_all(watch_dir)?;
    fs::create_dir_all(synced_dir)?;

    info!("Starte Dateiüberwachung für: {}", watch_dir);

    // Erstelle einen Channel für Dateiänderungen
    let (tx, rx) = channel();

    // Erstelle einen Watcher
    let mut watcher = watcher(tx, Duration::from_secs(2))?;

    // Überwache den watch-Ordner
    watcher.watch(watch_dir, RecursiveMode::Recursive)?;

    info!("Überwachung läuft. Drücke Ctrl+C zum Beenden.");

    // Verarbeite Änderungen
    loop {
        match rx.recv() {
            Ok(notify::DebouncedFileSystemEvent::Create(path)) => {
                info!("Neue Datei erkannt: {:?}", path);
                move_file_to_synced(&path, synced_dir)?;
            }
            Ok(notify::DebouncedFileSystemEvent::Write(path)) => {
                info!("Datei geändert: {:?}", path);
                if path.is_file() {
                    move_file_to_synced(&path, synced_dir)?;
                }
            }
            Ok(_) => {}
            Err(e) => error!("Fehler beim Überwachen: {}", e),
        }
    }
}

fn move_file_to_synced(file_path: &Path, synced_dir: &str) -> std::io::Result<()> {
    if file_path.is_file() {
        let file_name = file_path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown");

        let dest_path = PathBuf::from(synced_dir).join(file_name);

        fs::copy(file_path, &dest_path)?;
        fs::remove_file(file_path)?;

        info!("Datei verschoben: {} -> {}", file_path.display(), dest_path.display());
    }
    Ok(())
}

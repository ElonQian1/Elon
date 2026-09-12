use std::{
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};

fn run() -> anyhow::Result<()> {
    let args: Vec<_> = std::env::args_os().skip(1).collect();
    anyhow::ensure!(
        args.len() == 4,
        "usage: esk-game-reconciler CONFIG REQUEST EVIDENCE_DIRECTORY NEW_OUTPUT"
    );
    let paths: Vec<_> = args.into_iter().map(PathBuf::from).collect();
    let now_ms = i64::try_from(SystemTime::now().duration_since(UNIX_EPOCH)?.as_millis())?;
    esk_game_reconciler::files::run(&paths[0], &paths[1], &paths[2], &paths[3], now_ms)?;
    println!("{{\"status\":\"unsigned_candidate_saved\",\"funds_moved\":false}}");
    Ok(())
}

fn main() {
    if run().is_err() {
        // Do not echo account identifiers, archive contents or arbitrary input errors.
        eprintln!("reconciliation candidate rejected; verify configuration, signatures, periods, evidence archive and a new output path");
        std::process::exit(1);
    }
}

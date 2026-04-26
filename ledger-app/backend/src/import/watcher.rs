use anyhow::Result;
use notify::{EventKind, RecommendedWatcher, RecursiveMode, Watcher};

use crate::{AppState, import};

pub async fn spawn_import_watcher(state: AppState) -> Result<()> {
    let dir = state.config.import_dir.clone();
    if !dir.exists() {
        tokio::fs::create_dir_all(&dir).await?;
    }

    tokio::spawn(async move {
        if let Err(err) = run_watcher(state, dir).await {
            tracing::error!(error = ?err, "가져오기 폴더 감시 중 오류");
        }
    });

    Ok(())
}

async fn run_watcher(state: AppState, dir: std::path::PathBuf) -> Result<()> {
    let (tx, mut rx) = tokio::sync::mpsc::channel(128);

    let mut watcher: RecommendedWatcher = notify::recommended_watcher(move |res| {
        let _ = tx.blocking_send(res);
    })?;

    watcher.watch(&dir, RecursiveMode::NonRecursive)?;
    tracing::info!(path = %dir.display(), "가져오기 폴더 감시 시작");

    while let Some(event) = rx.recv().await {
        match event {
            Ok(event) => {
                let relevant = matches!(event.kind, EventKind::Create(_) | EventKind::Modify(_));
                if !relevant {
                    continue;
                }

                for path in event.paths {
                    if !path.is_file() {
                        continue;
                    }
                    let ext = path
                        .extension()
                        .map(|v| v.to_string_lossy().to_lowercase())
                        .unwrap_or_default();
                    if !matches!(ext.as_str(), "csv" | "xls" | "xlsx") {
                        continue;
                    }

                    match import::first_user_id(&state.pool).await {
                        Ok(Some(user_id)) => {
                            if let Err(err) =
                                import::process_file_from_path(&state.pool, user_id, &path).await
                            {
                                tracing::warn!(
                                    file = %path.display(),
                                    error = %err,
                                    "감시 폴더 파일 처리 실패"
                                );
                            } else {
                                tracing::info!(file = %path.display(), "감시 폴더 파일 미리보기 생성 완료");
                            }
                        }
                        Ok(None) => {
                            tracing::warn!("사용자가 없어 감시 폴더 가져오기를 건너뜁니다");
                        }
                        Err(err) => {
                            tracing::warn!(error = %err, "기본 사용자 조회 실패");
                        }
                    }
                }
            }
            Err(err) => {
                tracing::warn!(error = %err, "파일 감시 이벤트 오류");
            }
        }
    }

    Ok(())
}

use crate::auth;

use anyhow::anyhow;

lazy_static! {
    static ref QUERIES: Vec<&'static str> = vec![include_str!("1.sql"),];
}

pub async fn migrate<P: AsRef<std::path::Path>>(path: P) -> Result<(), anyhow::Error> {
    let conn = match rusqlite::Connection::open(path.as_ref()) {
        Ok(conn) => conn,
        Err(e) => {
            return Err(anyhow!("failed open database file: {}", e));
        }
    };

    let user_version: i32 = conn
        .pragma_query_value(Some(rusqlite::DatabaseName::Main), "user_version", |row| {
            row.get(0)
        })
        .unwrap_or(0);
    info!("Schema version {}", user_version);

    if QUERIES.len() > user_version as usize {
        info!("Schema version mismatch");
        for (i, query) in QUERIES.iter().enumerate() {
            if i + 1 > user_version as usize {
                info!("Migrating {}", i + 1);
                if let Err(e) = conn.execute_batch(query) {
                    return Err(anyhow!("failed: {}", e));
                }
            }
        }

        if user_version == 0 {
            let auth = auth::auth::Auth::new(path.as_ref().to_str().unwrap().to_string());
            auth.register(auth::User {
                username: "admin".to_string(),
                password: Some("admin".to_string()),
                role: "ADMIN".to_string(),
            })
            .await;
        }

        if let Err(e) = conn.pragma_update(
            Some(rusqlite::DatabaseName::Main),
            "user_version",
            &(QUERIES.len() as i32),
        ) {
            return Err(anyhow!("error set PRAGMA user_version: {}", e));
        }
    }
    Ok(())
}
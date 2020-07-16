use anyhow::Result;
use argon2::{self, Config};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use rand;
use rand::Rng;
use rusqlite::{params, Connection};

use crate::auth::{Claims, User, UserResponse};

#[derive(Clone)]
pub struct Auth {
    database_path: String,
}

impl Auth {
    pub fn new(database_path: String) -> Self {
        Self { database_path }
    }

    fn connect_db(&self) -> Connection {
        Connection::open(self.database_path.clone()).unwrap()
    }

    pub async fn register(&self, user: User) -> UserResponse {
        let db = self.connect_db();
        let hashed = Auth::hash(user.password.unwrap_or("tanoshi123".to_string()).as_bytes());
        match db.execute(
            r#"INSERT INTO "user"(username, password, role) VALUES (?1, ?2, ?3)"#,
            params![user.username, hashed, user.role],
        ) {
            Ok(_) => UserResponse {
                claim: None,
                token: None,
                status: "success".to_string(),
            },
            Err(e) => UserResponse {
                claim: None,
                token: None,
                status: format!("failed create account, reason: {}", e.to_string()),
            },
        }
    }

    pub async fn login(&self, secret: String, user: User) -> UserResponse {
        let db = self.connect_db();
        let account = db
            .query_row(
                r#"SELECT username, password, role FROM "user" WHERE username = ?1"#,
                params![user.username],
                |row| {
                    Ok(User {
                        username: row.get(0)?,
                        password: row.get(1)?,
                        role: row.get(2)?,
                    })
                },
            )
            .unwrap();

        if Auth::verify(account.password.unwrap(), user.password.unwrap().as_bytes()) {
            let user_claims = Claims {
                sub: account.username,
                role: account.role,
                exp: 10000000000,
            };
            let token = match encode(
                &Header::default(),
                &user_claims,
                &EncodingKey::from_secret(secret.as_bytes()),
            ) {
                Ok(t) => t,
                Err(_) => panic!(), // in practice you would return the error
            };
            return UserResponse {
                claim: Some(user_claims),
                token: Some(token),
                status: "success".to_string(),
            };
        }
        return UserResponse {
            claim: None,
            token: None,
            status: "failed".to_string(),
        };
    }

    pub async fn user_list(&self) -> Vec<User> {
        let db = self.connect_db();
        let mut stmt = db.prepare(r#"SELECT username, role FROM "user""#).unwrap();
        stmt.query_map(params![], |row| {
            Ok(User {
                username: row.get(0)?,
                role: row.get(1)?,
                password: None,
            })
        })
        .unwrap()
        .filter_map(|u| u.ok())
        .collect()
    }

    pub async fn change_password(&self, username: String, password: String) -> Result<()> {
        let db = self.connect_db();
        let hashed = Auth::hash(password.as_bytes());
        db.execute(
            r#"UPDATE "user" SET password = ?1 WHERE username = ?2"#,
            params![hashed, username],
        )?;
        Ok(())
    }

    pub async fn modify_user_role(&self, user: User) -> Result<()> {
        let db = self.connect_db();
        db.execute(
            r#"UPDATE "user" SET role = ?1 WHERE username = ?2"#,
            params![user.role, user.username],
        )?;
        Ok(())
    }

    pub fn validate(secret: String, token: String) -> Option<Claims> {
        match decode::<Claims>(
            &token,
            &DecodingKey::from_secret(secret.as_bytes()),
            &Validation::default(),
        ) {
            Ok(claim) => Some(claim.claims),
            Err(_) => None,
        }
    }

    fn hash(password: &[u8]) -> String {
        let salt: [u8; 32] = rand::thread_rng().gen();
        let config = Config::default();
        return argon2::hash_encoded(password, &salt, &config).unwrap();
    }

    fn verify(hash: String, password: &[u8]) -> bool {
        return argon2::verify_encoded(&hash, password).unwrap();
    }
}

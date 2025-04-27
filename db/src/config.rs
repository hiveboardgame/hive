use dotenvy::dotenv;
use std::{env, env::VarError};

#[derive(Clone, Debug)]
pub struct DbConfig {
    pub database_url: String,
    pub session_secret: String,
    pub jwt_secret: String,
}

impl DbConfig {
    pub fn from_env() -> Result<DbConfig, VarError> {
        dotenv().ok();
        Ok(DbConfig {
            database_url: env::var("DATABASE_URL")?,
            session_secret: env::var("COOKIE_SECRET_KEY")?,
            jwt_secret: env::var("JWT_SECRET_KEY")?,
        })
    }

    pub fn from_test_env() -> Result<DbConfig, VarError> {
        dotenv().ok();
        if cfg!(test) {
            return Ok(DbConfig {
                database_url: env::var("TEST_DATABASE_URL")?,
                session_secret: env::var("TEST_COOKIE_SECRET_KEY")?,
                jwt_secret: env::var("TEST_JWT_SECRET_KEY")?,
            });
        }
        unreachable!("You called a test function in a non test binary!");
    }
}

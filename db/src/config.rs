use dotenvy::dotenv;
use std::{env, env::VarError};

#[derive(Clone, Debug)]
pub struct DbConfig {
    pub database_url: String,
}

impl DbConfig {
    pub fn from_env() -> Result<DbConfig, VarError> {
        dotenv().ok();
        Ok(DbConfig {
            database_url: env::var("DATABASE_URL")?,
        })
    }

    pub fn from_test_env() -> Result<DbConfig, VarError> {
        dotenv().ok();
        if cfg!(test) {
            return Ok(DbConfig {
                database_url: env::var("TEST_DATABASE_URL")?,
            });
        }
        unreachable!("You called a test function in a non test binary!");
    }
}

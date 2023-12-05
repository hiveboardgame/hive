use db_lib::config::DbConfig;
use db_lib::get_pool;
use db_lib::models::user::{NewUser, User};

#[tokio::main]
async fn main() {
    let config = DbConfig::from_env().expect("Failed to load config from env");
    let pool = &get_pool(&config.database_url)
        .await
        .expect("Failed to get pool");
    let new_user = NewUser::new("leex", "hunter2", "leex").expect("Failed to make new_user");
    let user = User::create(&new_user, pool)
        .await
        .expect("Failed to create user");
    println!("User {:?}", user);
    let deleted = user.delete(pool).await.expect("Failed to delete user");
    println!("Deleted {:?}", deleted);
}

use db_lib::config::DbConfig;
use db_lib::models::user::User;
use db_lib::get_pool;

#[tokio::main]
async fn main() {
    let config = DbConfig::from_env().expect("Failed to load config from env");
    let pool = &get_pool(&config.database_url)
        .await
        .expect("Failed to get pool");
    let uuid = "unique";
    let user = User::new(uuid, "leex", "hunter2", "token").expect("Failed to make user");
    user.insert(pool).await.expect("Failed to insert User");
    println!("{:?}", user);
    let found = User::find_by_uid(uuid, pool)
        .await
        .expect("Couldn't find user");
    println!("{:?}", found);
    // let deleted = found.delete(pool).await.expect("Failed to delete user");
    // println!("{:?}", deleted);
}



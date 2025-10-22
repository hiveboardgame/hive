use crate::common::{ScheduleUpdate, ServerMessage};
use crate::responses::ScheduleResponse;
use crate::websocket::new_style::server::{ServerData, TabData};
use db_lib::get_conn;
use db_lib::models::Schedule;

pub async fn send_schedules(client: &TabData, server: &ServerData) {
    let mut conn = match get_conn(client.pool()).await {
        Ok(conn) => conn,
        Err(_) => {
            println!("Failed to get connection for schedules");
            return;
        }
    };

    if let Some(account) = client.account() {
        let user_id = account.id;

        if let Ok(schedules) = Schedule::find_user_notifications(user_id, &mut conn).await {
            for schedule in schedules {
                let is_opponent = schedule.opponent_id == user_id;

                if let Ok(response) = ScheduleResponse::from_model(schedule, &mut conn).await {
                    let update = if is_opponent {
                        ScheduleUpdate::Proposed(response)
                    } else {
                        ScheduleUpdate::Accepted(response)
                    };

                    client.send(ServerMessage::Schedule(update), server);

                }
            }
        }
    }
}

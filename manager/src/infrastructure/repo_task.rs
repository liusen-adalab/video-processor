use utils::db_pools::postgres::PgConn;

use crate::manager::Task;

pub async fn save(task: &Task, conn: &mut PgConn) -> anyhow::Result<()> {
    todo!()
}

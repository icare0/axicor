use crate::node::NodeRuntime;

impl NodeRuntime {
    /// Вызывается, если BspBarrier зафиксировал смерть соседа.
    /// [TODO] Переписать под новый headerless SoA формат реплик.
    pub async unsafe fn resurrect_shard(&self, dead_zone_hash: u32) {
        println!("[Recovery] Resurrection 0x{:08X} skipped (Legacy Recovery is disabled)", dead_zone_hash);
    }

    pub async fn broadcast_route_update(&self, _zone_hash: u32) {
        // Заглушка
    }
}

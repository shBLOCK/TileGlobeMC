use uuid::Uuid;

pub trait MCPlayerUUID {
    fn new_mc_offline_player(player_name: &str) -> Uuid {
        let mut hasher = md5::Context::new();

        hasher.consume("OfflinePlayer:".as_bytes());
        hasher.consume(player_name.as_bytes());

        uuid::Builder::from_md5_bytes(*hasher.finalize()).into_uuid()
    }
}

impl MCPlayerUUID for Uuid {}

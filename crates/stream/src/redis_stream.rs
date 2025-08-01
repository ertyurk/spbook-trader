// Redis streaming implementation

use redis::Client;
use anyhow::Result;

pub struct RedisStream {
    client: Client,
}

impl RedisStream {
    pub async fn new(redis_url: &str) -> Result<Self> {
        let client = Client::open(redis_url)?;
        Ok(Self { client })
    }
}
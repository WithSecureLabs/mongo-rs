use mongod;

#[allow(dead_code)]
pub fn setup() {
    // FIXME: Until we impl database and collection on the blocking client we have to spawn in a
    // runtime...
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async_setup());
}

pub async fn async_setup() {
    let client = mongod::Client::new();
    client.database().drop(None).await.unwrap();
}

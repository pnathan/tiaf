use tiaf;

use std::sync::{Arc, RwLock};

struct TestContext {}
impl TestContext {
    fn new() -> TestContext {
        TestContext {}
    }
}
impl Drop for TestContext {
    fn drop(&mut self) {
        println!("Dropping TestContext");
    }
}

#[test]
fn test_add() {
    let blockchain = Arc::new(RwLock::new(tiaf::chain::Blockchain::new()));
    let mem_pool = Arc::new(RwLock::new(tiaf::mempool::MemPool::new(8)));

    // https://docs.rs/rouille/latest/rouille/struct.Server.html#method.stoppable
    // tiaf::server::launch_server(blockchain, mem_pool);
}

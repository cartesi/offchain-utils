use block_subscriber::{BlockSubscriber, NewBlockSubscriber};
use middleware_factory::WsProviderFactory;
use offchain_core::ethers::core::utils::Geth;

#[tokio::test]
async fn subscribe_test() {
    let geth = Geth::new().block_time(1u64).spawn();
    let factory = WsProviderFactory::new(
        geth.ws_endpoint(),
        0,
        std::time::Duration::from_secs(1),
    )
    .await
    .unwrap();

    let (block_subscriber, handle) = BlockSubscriber::create_and_start(
        factory,
        std::time::Duration::from_secs(15),
        0,
        std::time::Duration::from_secs(1),
    );

    let mut subscription = block_subscriber.subscribe().await.unwrap();
    let mut current_block = subscription.recv().await.unwrap().number;
    for _ in 0u64..16 {
        let head = subscription.recv().await.unwrap();
        let new_block = head.number;
        assert_eq!(current_block + 1, new_block);
        current_block = new_block;
    }

    handle.kill_switch.send(()).unwrap();
    handle.handle.await.unwrap().unwrap();

    assert!(block_subscriber.subscribe().await.is_none());
    assert!(subscription.recv().await.is_err());
}

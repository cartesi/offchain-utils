pub mod contract;

pub use ethabi;
pub use ethers;

pub mod types {
    use serde::{Serialize, Deserialize};

    #[derive(Clone, Debug, Serialize, Deserialize)]
    pub struct Block {
        pub hash: ethers::types::H256,
        pub number: ethers::types::U64,
        pub parent_hash: ethers::types::H256,
        pub timestamp: ethers::types::U256,
        pub logs_bloom: ethers::types::Bloom,
    }

    impl<T> std::convert::TryFrom<ethers::types::Block<T>> for Block {
        type Error = String;
        fn try_from(b: ethers::types::Block<T>) -> Result<Self, Self::Error> {
            Ok(Self {
                hash: b.hash.ok_or("Block has no hash")?,
                number: b.number.ok_or("Block has no number")?,
                parent_hash: b.parent_hash,
                timestamp: b.timestamp,
                logs_bloom: b.logs_bloom.ok_or("Block has no logs bloom")?,
            })
        }
    }
}

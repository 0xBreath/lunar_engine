use std::collections::BTreeMap;
use std::io::Result;
use std::time::{SystemTime, UNIX_EPOCH};

#[allow(dead_code)]
pub struct AllOrders {}

impl AllOrders {
    #[allow(dead_code)]
    pub fn request(symbol: String) -> String {
        Self::create_request(symbol)
    }

    pub fn get_timestamp() -> Result<u64> {
        let system_time = SystemTime::now();
        let since_epoch = system_time
            .duration_since(UNIX_EPOCH)
            .expect("System time is before UNIX EPOCH");
        Ok(since_epoch.as_secs() * 1000 + u64::from(since_epoch.subsec_nanos()) / 1_000_000)
    }

    #[allow(dead_code)]
    fn build(symbol: String) -> BTreeMap<String, String> {
        let mut btree = BTreeMap::<String, String>::new();
        btree.insert("symbol".to_string(), symbol);
        let timestamp = Self::get_timestamp().expect("Failed to get timestamp");
        btree.insert("timestamp".to_string(), timestamp.to_string());
        btree
    }

    #[allow(dead_code)]
    fn create_request(symbol: String) -> String {
        let btree = Self::build(symbol);
        let mut request = String::new();
        for (key, value) in btree.iter() {
            request.push_str(&format!("{}={}&", key, value));
        }
        request.pop();
        request
    }
}

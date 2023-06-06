use std::collections::BTreeMap;

pub struct Price {
    /// Ticker symbol (e.g. BTCUSDC)
    pub symbol: String,
    pub recv_window: Option<u32>
}

impl Price {
    pub fn request(symbol: String, recv_window: Option<u32>) -> String {
        let me = Self { symbol, recv_window };
        me.create_request()
    }

    fn build(&self) -> BTreeMap<String, String> {
        let mut btree = BTreeMap::<String, String>::new();
        btree.insert("symbol".to_string(), self.symbol.to_string());
        if let Some(recv_window) = self.recv_window {
            btree.insert("recvWindow".to_string(), recv_window.to_string());
        }
        btree
    }

    fn create_request(&self) -> String {
        let btree = self.build();
        let mut request = String::new();
        for (key, value) in btree.iter() {
            request.push_str(&format!("{}={}&", key, value));
        }
        request.pop();
        request
    }
}

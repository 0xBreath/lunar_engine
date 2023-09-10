#[cfg(tests)]
mod tests {
    #[test(tokio::test)]
    fn stock_stream() {
        let api_info = ApiInfo::from_parts(
            ALPACA_API_LIVE_URL,
            ALPACA_LIVE_API_KEY,
            ALPACA_LIVE_API_SECRET,
        )?;
        let client = Client::new(api_info);

        let (mut stream, mut subscription) = client.subscribe::<RealtimeData<IEX>>().await?;
        let mut data = MarketData::default();
        data.set_bars(["SPY"]);
        let subscribe = subscription.subscribe(&data).boxed();
        let () = drive(subscribe, &mut stream)
            .await
            .unwrap()
            .unwrap()
            .unwrap();

        let read = stream
            .map_err(Error::WebSocket)
            .try_for_each(|result| async {
                let res = result.map_err(Error::Json);
                info!("{:?}", res);
                Ok(())
            });
        info!("Starting stock stream...");

        match read.await {
            Ok(()) => info!("done"),
            Err(e) => error!("error: {}", e),
        };
    }

    #[test(tokio::test)]
    fn crypto_stream() {
        let api_info = ApiInfo::from_parts(
            ALPACA_API_LIVE_URL,
            ALPACA_LIVE_API_KEY,
            ALPACA_LIVE_API_SECRET,
        )?;
        let client = Client::new(api_info);

        let (mut stream, mut subscription) = client
            .subscribe::<RealtimeData<CustomUrl<Crypto>>>()
            .await
            .unwrap();
        let mut data = MarketData::default();
        data.set_bars(["BTC/USD"]);

        let subscribe = subscription.subscribe(&data).boxed();
        // Actually subscribe with the websocket server.
        let () = drive(subscribe, &mut stream)
            .await
            .unwrap()
            .unwrap()
            .unwrap();

        let read = stream
            .map_err(Error::WebSocket)
            .try_for_each(|result| async {
                let res = result.map_err(Error::Json);
                info!("{:?}", res);
                Ok(())
            });
        info!("Starting stock stream...");

        match read.await {
            Ok(()) => info!("done"),
            Err(e) => error!("error: {}", e),
        };
    }
}

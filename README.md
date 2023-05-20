# Lunar Engine

### Steps to deploy to Heroku:
Heroku is connected to `lunar_engine` main branch. Any push to `main` will trigger a deploy to Heroku.
If deploying to a new Heroku app, configure the buildpack with Rust using:
```shell
heroku buildpacks:set emk/rust
```
Make sure a `Procfile` exists in the root directory with the following contents:
```shell
web: target/release/server_app
```
The Heroku app should be set up to deploy with every push to `main` branch.
If it does not, navigate to the Heroku website and manually deploy the app.
Avoid deploying from the CLI to keep things simple.

### Steps to automate trade alerts
Add strategy to Tradingview chart

Add this webhook to the alert: `https://lunar-engine.herokuapp.com/alert`

Add this to the message field of the alert: `{{strategy.order.alert_message}}`

The server expects the alert message in string format, NOT JSON.
Example:
`{side: Long, order: Enter, timestamp: 1680381231283}`
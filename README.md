# Lunar Engine

### Steps to deploy to Heroku:
Heroku is connected to `lunar_engine` main branch. Any push to `main` will trigger a deploy to Heroku.
If deploying to a new Heroku app, configure the buildpack with Rust using:
```shell
heroku buildpacks:set emk/rust
```
Make sure a `Procfile` exists in the root directory with the following contents:
```shell
web: target/release/server
```
The Heroku app should be set up to deploy with every push to `main` branch.
If it does not, navigate to the Heroku website and manually deploy the app.
Avoid deploying from the CLI to keep things simple.
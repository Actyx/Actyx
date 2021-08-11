# Actyx on Android

## Code style
To format the code, we use [ktlint](https://github.com/shyiko/ktlint)

Here are the commands:
* To check files format: `./gradlew ktlint`
* To format files: `./gradlew ktlintFormat`

## Automatic Versioning

The `versionCode` is basically the number of commits in the git repository (plus 102011000 to not interfere with already released versions.)

The `versionName` of the APK is either the value of `$ACTYX_VERSION` environment variable if it is set or "0.0.0_dev-" plus the git hash of the current `HEAD` with a dirty marker.

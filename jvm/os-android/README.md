# Actyx on Android

## Code style
To format the code, we use [ktlint](https://github.com/shyiko/ktlint)

Here are the commands:
* To check files format: `./gradlew ktlint`
* To format files: `./gradlew ktlintFormat`

## Automatic Versioning

Now that we bundle up the store together with the Android Shell App, versioning it becomes more important.
The Android Shell app now uses the gradle plugin `gradle-android-git-version` with the following configuration:

```
✗ ./gradlew --quiet androidGitVersion
androidGitVersion.name  2.0.1-2<9fd1296>-ow_android-versioning-dirty
androidGitVersion.code  200002002
```

The version code is derived by the
	* the major version (2),
	* minor version (00),
	* patch version (002),
	* the number of commits since the last tag (002).

The version of the app is derived from a git tag, prefixed with `axos-android-`, e.g. `ax-osandroid-2.0.2`. **Please tag the last commit of your PR accordingly.**

The name is derived by:
	* The version, as taken from the tag (2.0.1)
	* the number of commits since the last tag (2)
	* the hash of the commit
	* the branch name (omitted on master)
	* a dirty flag, indicating whether there are uncomitted changes.

### Tagging when squashing and merging a PR

You can't really tag a squashed commit. Workaround:

1. Merge to master
2. Pull locally
3. Tag locally - `git tag xzy axos-android-2.xxx.x`
4. Push tag to remote - `git push origin axos-android-2.xxx.x`

**Do that before the Android build on azure pipelines starts or restart it.**

TODO: Badges for nuget and docs

# <img src="https://developer.actyx.com/img/logo.svg" height="32px"> ActyxOS SDK

[ActyxOS](https://developer.actyx.com/docs/os/introduction) makes it easy to run distributed
applications on multiple nodes. It is a piece of software that allows you to run your own apps
on one or more edge devices and have these apps seamlessly communicate and share data with
each other.

This project defines the data types needed for communicating with ActyxOS and provides C#
bindings for the ActyxOS APIs.

## Examples

TODO

## Publishing to NuGet

run

```
dotnet nuget push \
  Sdk/bin/<path_to_artifact>.nupkg \
  --api-key $(vault kv get -field=api_key secret/ops.actyx/nuget) \
  --source https://api.nuget.org/v3/index.json
```
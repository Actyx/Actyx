TODO: link to docs

[![NuGet](https://buildstats.info/nuget/Actyx.Sdk)](https://www.nuget.org/packages/Actyx.Sdk/)

# <img src="https://developer.actyx.com/img/logo.svg" height="32px"> Actyx SDK

[Actyx](https://www.actyx.com/platform) makes it easy to run distributed
applications on multiple nodes. It is a piece of software that allows you to run your own apps
on one or more edge devices and have these apps seamlessly communicate and share data with
each other.

This project defines the data types needed for communicating with Actyx and provides C#
bindings for the Actyx APIs.

## Examples

TODO

## Releasing

[GitVersion](https://gitversion.net/) is used to automatically set the version based on git tags. The prefix used is `"dotnet/sdk-"`. E.g. version `1.0.0` is created by running

```
git tag dotnet/sdk-1.0.0 <optional refspec, otherwise HEAD is used>
```

## Building

```bash
dotnet pack --configuration Release # default config is Debug
```

## Publishing to NuGet

```
dotnet nuget push \
  Sdk/bin/<path_to_artifact>.nupkg \
  --api-key $(vault kv get -field=api_key secret/ops.actyx/nuget) \
  --source https://api.nuget.org/v3/index.json
```
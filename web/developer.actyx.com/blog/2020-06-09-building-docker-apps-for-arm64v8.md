---
title: Building Docker apps for arm64
author: Maximilian Haushofer
author_title: Product Manager at Actyx
author_url: https://www.linkedin.com/in/maximilianhaushofer/
author_image_url: https://images.ctfassets.net/55iqf6xnllwu/7exkxedRNkZNPjeWIJAJFy/7f238372c06ddfc64fa321d5e665dc62/maximilian-haushofer.jpg
tags: [ActyxOS, arm64 ]
---

Want to deploy your Docker app to a device running on arm64? Check out this blog post.

<!--truncate-->

As part of the ActyxOS 1.0.0-rc.2 release, we included support for devices running on `arm64`. The corresponding release of the Actyx CLI also made it possible to package your Docker app for multiple architectures at the same time. In this blog post, we'll take a look at the build procedure for apps that need to be deployed to devices running on `arm64` such as the RPi 3 or 4.

If you want to follow the steps in this guide, you'll need a few things:

- development machine running Docker
- a device running an arm64 operating system, and ActyxOS on Docker ([check out our installation guide](/docs/os/advanced-guides/actyxos-on-docker#install-actyxos-on-your-edge-device))
- a Dockerfile for the image(s) you want to package as an app
- basic knowledge of how building ActyxOS apps works (check out our guide on [building apps](/docs/os/guides/building-apps))

:::note Terminology
In our documentation, as well as in the Docker documentation, you will sometimes read different terms that actually refer to the same architecture. Be aware that, in this context, `x86_64` and `amd64` mean the same thing, and so do `arm64v8`, `arm64` and `aarch64`.
:::

### 1. Build your Docker image

You first need to make sure that your Docker image(s) is actually built for the correct architecture. By default,  Docker images built on your development machine are built for the architecture of your development machine – which usually is `x86_64`.

If you are a proficient Docker user and running macOS or Linux, you can just use [docker buildx](https://docs.docker.com/buildx/working-with-buildx/) to build an image for another architecture.

If that is not the case, we recommend using our [Windows Cross Builder](https://hub.docker.com/repository/docker/actyx/windows-cross-builder). It is a easy-to-use tool we created for building Docker images for multiple architectures. Don't worry if you are running macOS or Linux – although it's called Windows Cross Builder, it will work on all host operating systems.

Assuming the absolute path to the folder containing your Dockerfile is `/Users/user/sample-docker-app/`, run the following command:

```bash
docker run --privileged -e IMAGE_TAG="sample-docker-app-arm64" -e PLATFORM="linux/arm64" --rm -v /Users/user/sample-docker-app/:/data actyx/windows-cross-builder
```

:::info ActyxOS on Docker only supports x86_64 and arm64
Although the Windows Cross Builder allows you to build images for a number of architectures, ActyxOS currently only supports x86_64 and arm64.
:::

After the Windows Cross Builder finished, you will find a file called `sample-docker-app-arm64.tar.gz` in the same folder that also contains your Dockerfile. In order to be able to reference the image in your docker-compose.yml file, you need to load it into your local Docker instance:

```bash
docker load -i sample-docker-app-arm64.tar.gz
```

If you want to check whether the Docker image was successfully loaded, just run `docker image ls` .

### 2. Package your ActyxOS app

After following the above steps, you can reference this image in the docker-compose file that you will need for packaging your ActyxOS app (check out [this guide](/docs/os/guides/building-apps) for more info). Your app manifest should now look similar to this:

```yml
manifestVersion: "1.0"
id: com.sample.myapp
version: 1.0.0
displayName: Sample Docker App
description: "An Sample Docker app for arm64"
settingsSchema: ./settings-schema.json
type: docker
dockerCompose:
  aarch64: ./docker-compose-arm64.yml # this is the path to the docker-compose file for your app
```

After executing `ax apps package`, you can find the file `com.actyx.sample-docker-app-1.0.0-aarch64.tar.gz` in the folder from which you executed the command.

**Your app is now** [**ready to be deployed on ActyxOS on Docker**](/docs/os/guides/running-apps) **running on arm64.**

:::tip Also packaging for x86_64 devices?
The Actyx CLI allows you to package an app for multiple architectures at the same time – just specify the other docker-compose file in the same app manifest, and ax apps package will return two tarballs containing the same app for different architectures. Check out [this example](/docs/os/api/app-manifest-schema).
:::

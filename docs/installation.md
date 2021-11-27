# Installation

There are 3 ways to use Tanoshi:
- Using prebuild binary
- Using official Docker image
- Using desktop version

## Prebuilt Binary
Download and run binary from latest release, aside from plugins all dependencies are statically linked. Prebuilt binary available for `amd64` platform.

## Docker
Docker image available for `amd64` and `arm64`. If you want to selfhost on a raspberry pi, you need 64 bit OS, such as ubuntu or Raspbian OS 64 bit. Tanoshi can't run on `arm` because [wasmer](https://github.com/wasmerio/wasmer) can't run on those platform yet, when `wasmer` could run on `arm` I will build image for `arm`.

If you want to use unreleased features, you can use tag `master`. It has everything from `master` branch.

```
docker create \
    --name=tanoshi \
    -p 8080:80 \
    --mount type=bind,source=/path/to/data,target=/tanoshi \
    --restart unless-stopped \
    faldez/tanoshi
```

```
docker start tanoshi
```

## Docker-compose

Refer to docker-compose.yaml.

## Desktop
Download `.msi` for windows, `.deb` or `.AppImage` for linux, `.dmg` for mac from latest release to download desktop version.

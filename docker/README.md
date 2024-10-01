# Ubuntu-Rust-Gtk4 Docker Image

- base image: `Ubuntu:oracular`
- installed packages: see Dockerfile
- usage:

```
docker run --rm --platform [linux/amd64] -v ${{ github.workspace }}:/app -v ./entrypoint.sh:/entrypoint.sh [image]
```

> [!NOTE]  
> use your own `entrypoint.sh` and mount `/app` to your project root dir. need `sudo` privilege to move `target/`

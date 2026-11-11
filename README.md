# MyBatis Navigation for Zed

An offline, self-contained MyBatis navigation extension for Zed.

## Features

- Java Mapper method → XML `<select>`, `<insert>`, `<update>`, or `<delete>`.
- XML statement `id` → Java Mapper method.
- Java Mapper interface ↔ XML Mapper `namespace`.
- Clickable `↗ XML` and `↗ Mapper` CodeLens links.
- Standard Zed **Go to Definition** support.

The Rust language server scans the open worktree and skips `.git`, `.idea`, `.zed`, `.gradle`, `node_modules`, `target`, and `build`.

## Offline runtime

This package does not execute Python, call GitHub, query a package registry, or download a language server.

The current complete package embeds a macOS Apple Silicon (`aarch64`) Rust `mybatis-lsp` binary in the extension WASM. On first start, Zed writes that binary into the extension work directory and launches it directly.

> This artifact is intentionally for macOS Apple Silicon only. A Windows, Linux, or Intel Mac package must embed a binary compiled for that target before installation.

## Supported conventions

```java
package example.mapper;
public interface UserMapper {
    int deleteByPrimaryKey(Long id);
}
```

```xml
<mapper namespace="example.mapper.UserMapper">
  <delete id="deleteByPrimaryKey">...</delete>
</mapper>
```

Not yet supported: annotation SQL, overloaded Mapper methods, `<sql>` / `<include>` navigation, and `resultMap` navigation.

## Build the complete macOS Apple Silicon package

```sh
cargo build --release --manifest-path lsp/Cargo.toml
cp target/release/mybatis-lsp server/mybatis-lsp-macos-aarch64
cargo build --release --target wasm32-wasip2
```

Then run **`zed: Install Dev Extension`** and select this directory.

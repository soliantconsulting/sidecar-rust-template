# Sidecar Rust Template

This template will generate a Sidecar written in Rust with an HTTP endpoint and queue processing.

## Usage

If not already done, install `cargo-generate`:

```bash
cargo install cargo-generate
```

Now you can generate a new sidecar with the following command. Please note when choosing a project that it must start
with "sidecar-".

```bash
cargo generate gh:soliantconsulting/sidecar-rust-template
```

After creation, change into the newly created directory and run `pnpm boot`.

# rust-benchmark-glibc-musl-mimalloc

## How memory allocation works on Linux

```mermaid
flowchart TD
  app[Application code]
  std[Rust std::alloc]
  ga["#[global_allocator]<br/>jemalloc / mimalloc / system"]
  libc["libc malloc<br/>(ptmalloc on glibc, mallocng on musl)"]
  kernel["Kernel mmap / brk / sbrk"]
  phys[Physical memory]
  app --> std --> ga --> libc --> kernel --> phys
```

When a Rust program calls `Vec::new()` or `Box::new(x)`, the request travels through `std::alloc` → the configured `#[global_allocator]` (jemalloc / mimalloc / system) → libc malloc (ptmalloc on glibc, mallocng on musl) → the kernel's `mmap` / `brk` / `sbrk` → physical memory. Each layer can change the cost, fragmentation profile, and tail-latency shape of an allocation. This benchmark measures those differences across four allocators, six libc·env combinations, and eleven workload scenarios.


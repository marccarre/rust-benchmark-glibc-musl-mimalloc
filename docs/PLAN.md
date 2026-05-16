# Plan

Create a Rust benchmark for glibc (ptmalloc), musl (mallocng), jemalloc, and mimalloc.

- 1. Create either one CLI/crate with multiple tests that can be picked via CLI
  flags/options, OR a multi-crate workspace with one benchmark per crate,
  depending on what is most appropriate.
- 2. Have at least one benchmark optimal to compare glibc/musl/mimalloc, e.g.,
  just spawning a lot of the threads which each allocate a lot of objects of
  different sizes (how many objects and how large is each object could be CLI
  options).
- 3. Also have more "realistic" benchmarks, e.g.:
  - a. A web-service that ser/de a lot of requests, in various threads, to
    serve requets in parallel.
  - b. A spmc type of use-case/benchmark.
  - c. A mpsc type of use-case/benchmark.
  - d. A mpmc type of use-case/benchmark.
  - e. An example of algorithm that uses a lot of CPU.
  - f. An example of algorithm that uses a lot of memory.
  - g. Anything else that you deem appropriate/relevant.
- 4. All benchmarks should print the version of the compiler used, which,
  ideally, should be injected at compilation time.
- 5. For this benchmark, ensures we compile with the best options for the
  highest level of performance and the highest level of optimisations.
- 6. Benchmarks can measure either:
  - a. how long it takes to run the benchmark, AND/OR
  - b. how much can be computed per unit of time, e.g., how many requests can
    be handled per second, on an average of 1 minute.
- 7. Benchmarks shall have a "warm-up" run if/when necessary.
- 8. Benchmark in several environments:
  - a. the host, as-is,
  - b. Docker with alpine,
  - c. Docker with debian-slim,
  - d. Docker with distroless,
  - e. Docker with scratch,
  - f. any other small Docker image of relevance.
- 9. Show case best practices through everything done in this repository.
  For example, use Docker multi-stage builds, optimised images (e.g., with
  `dive`), [OCI labels] or the equivalent that the community may have converged
  on as of now (May 2026), etc.
- 10. It should be easy to interpret all results, e.g., in a Marimo notebook
  or a HTML page with Plotly diagrams, or equivalent, with the ability to
  visualise two or more benchmarks side-by-side, to slice and dice results,
  to filter by experiment or by runtime, etc.
- 11. After running the benchmarks, write a report with the overall analysis,
  a side-by-side comparison of the memory allocators evaluated, of the Docker
  runtimes evaluated, and recommendations if any.
- 12. In this report, create system diagrams in Mermaid.js to describe how each
  allocator works.
- 13. In the README.md of this repository create an overall system diagram of
  how memory allocation works on modern computers (in Mermaid.js).
- 14. Anything else that should have been suggested above?

[OCI labels]: https://github.com/opencontainers/image-spec/blob/main/annotations.md

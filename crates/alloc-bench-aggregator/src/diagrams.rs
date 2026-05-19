//! Mermaid top-down (TD) flowchart constants for the four benchmark
//! allocators (D-11, AGG-06).
//!
//! Each constant is a static `&'static str` containing a single fenced
//! `mermaid` code block — opening triple-backtick + `mermaid` info-string,
//! the top-down keyword, ~10–15 node/edge lines, closing triple-backtick.
//! The aggregator emits these blocks verbatim into REPORT.md by iterating
//! `ALL_DIAGRAMS` (alphabetical order: jemalloc, mallocng, mimalloc,
//! ptmalloc) per RESEARCH §Pitfall 5 byte-identical-output rules.
//!
//! Sources are cited per allocator as `/// Source: <url>` doc comments.
//! Diagrams are at "Wikipedia summary" level — they change only when our
//! understanding of the allocator changes, NOT per benchmark run (D-11).
//! UI-SPEC §Mermaid Theme Contract locks: default theme, no styling, plain
//! text edge labels, server-side renderer (GitHub / VS Code preview); we
//! do NOT bundle a Mermaid runtime in `report/index.html`.
//!
/// Source: https://en.wikipedia.org/wiki/C_dynamic_memory_allocation
pub const PTMALLOC_DIAGRAM: &str = r#"
```mermaid
flowchart TD
  thread[Thread] --> arena[Arena Selector]
  arena --> main[Main Arena: heap brk]
  arena --> per[Per-thread Arenas]
  main --> fast[Fastbins ≤ 80B]
  main --> small[Smallbins ≤ 504B]
  main --> large[Largebins ≤ 128KB]
  main --> unsorted[Unsorted Bin]
  main --> top[Top Chunk]
  large --> mmap[mmap chunks ≥ 128KB]
  per --> pfast[Per-arena Fastbins]
  per --> psmall[Per-arena Smallbins]
```
"#;

/// Source: https://github.com/richfelker/mallocng-draft
pub const MALLOCNG_DIAGRAM: &str = r#"
```mermaid
flowchart TD
  thread[Thread] --> classsel[Size Class Selector]
  classsel --> active[Active Group for Class]
  classsel --> newg[New Group from Heap]
  active --> slots[Group of N ≤ 32 slots]
  newg --> slots
  slots --> bitmap[Slot Bitmap Status]
  slots --> oob[Out-of-band Group Header]
  slots --> align[16-byte Base Alignment]
  classsel --> largealloc[Large Alloc 1-slot mmap Group]
```
"#;

/// Source: https://github.com/jemalloc/jemalloc/blob/dev/doc/jemalloc.xml.in
pub const JEMALLOC_DIAGRAM: &str = r#"
```mermaid
flowchart TD
  thread[Thread] --> tcache[Per-thread tcache]
  tcache --> arena[Arena: 4× num_cpus]
  arena --> binsmall[Bin Small Size Class]
  arena --> extent[Extent]
  binsmall --> slab[Slab in Extent]
  slab --> slot[Slot]
  slot --> bitmap[Slab Bitmap]
  arena --> largealloc[Large Alloc Dedicated Extent]
  tcache --> binmedium[Bin Medium 16/32/64B Intervals]
  binmedium --> slab
  arena --> quant[Page-size Quantization 2-4KiB]
```
"#;

/// Source: https://github.com/microsoft/mimalloc
pub const MIMALLOC_DIAGRAM: &str = r#"
```mermaid
flowchart TD
  thread[Thread] --> heap[Per-thread Heap]
  heap --> seg[Segment 64KB OS Alloc]
  seg --> page[Page: One Size Class]
  page --> tlfree[Thread-local Free List]
  page --> concfree[Concurrent Free List]
  concfree --> cas[Cross-thread Free via CAS]
  heap --> first[First-class Heap]
  heap --> arena[Arena for OS Reservations]
  page --> shard[Free-list Sharding]
  seg --> purge[Eager Page Purging]
  thread --> alloc[Alloc Hot Path]
  alloc --> tlfree
```
"#;

/// Alphabetical iteration order for emission into REPORT.md (D-09 byte-
/// identical-output contract; markdown.rs iterates this slice without
/// imposing its own ordering logic).
pub const ALL_DIAGRAMS: [(&str, &str); 4] = [
    ("jemalloc", JEMALLOC_DIAGRAM),
    ("mallocng", MALLOCNG_DIAGRAM),
    ("mimalloc", MIMALLOC_DIAGRAM),
    ("ptmalloc", PTMALLOC_DIAGRAM),
];

#[cfg(test)]
mod tests {
    use super::*;

    /// Each constant must contain exactly one fenced mermaid code block:
    /// opening fence (triple-backtick + info-string), `flowchart TD` line,
    /// at least 10 lines total, and a closing fence (so the count of
    /// triple-backtick fences equals 2).
    #[test]
    fn each_diagram_has_flowchart_td_and_min_nodes() {
        for (name, body) in ALL_DIAGRAMS.iter() {
            assert!(
                body.contains("flowchart TD"),
                "{name}: missing `flowchart TD` keyword"
            );
            assert!(
                body.contains("```mermaid"),
                "{name}: missing opening ```mermaid fence"
            );
            let fence_count = body.matches("```").count();
            assert_eq!(
                fence_count, 2,
                "{name}: expected exactly 2 triple-backtick fences (one open, one close), got {fence_count}"
            );
            let line_count = body.lines().count();
            assert!(
                line_count >= 10,
                "{name}: expected ≥ 10 lines, got {line_count}"
            );
        }
    }

    /// `ALL_DIAGRAMS` must be alphabetical (jemalloc, mallocng, mimalloc,
    /// ptmalloc). markdown.rs depends on this so it can iterate without
    /// imposing its own sort.
    #[test]
    fn diagrams_in_alphabetical_emission_order() {
        assert_eq!(ALL_DIAGRAMS[0].0, "jemalloc");
        assert_eq!(ALL_DIAGRAMS[1].0, "mallocng");
        assert_eq!(ALL_DIAGRAMS[2].0, "mimalloc");
        assert_eq!(ALL_DIAGRAMS[3].0, "ptmalloc");
    }
}

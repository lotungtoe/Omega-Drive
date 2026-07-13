use bytes::Bytes;
use std::collections::HashMap;

use super::*;

fn make_cache(max_bytes: Option<usize>) -> PartitionedMemCache {
    let mut cfg = HashMap::new();
    cfg.insert("test".into(), PartitionConfig { max_bytes });
    PartitionedMemCache::new(cfg)
}

#[tokio::test]
async fn test_write_and_read_exact() {
    let cache = make_cache(None);
    cache.write(1, 0, Bytes::from("hello"), "test").await;
    let data = cache.read(1, 0, 5).await;
    assert_eq!(data, Some(Bytes::from("hello")));
}

#[tokio::test]
async fn test_read_missing_returns_none() {
    let cache = make_cache(None);
    let data = cache.read(1, 0, 5).await;
    assert!(data.is_none());
}

#[tokio::test]
async fn test_write_empty_is_noop() {
    let cache = make_cache(None);
    cache.write(1, 0, Bytes::new(), "test").await;
    let data = cache.read(1, 0, 0).await;
    assert!(data.is_none());
}

#[tokio::test]
async fn test_read_subrange_of_entry() {
    let cache = make_cache(None);
    cache.write(1, 10, Bytes::from("abcdef"), "test").await;
    let data = cache.read(1, 12, 3).await;
    assert_eq!(data, Some(Bytes::from("cde")));
}

#[tokio::test]
async fn test_read_within_merged_entry() {
    let cache = make_cache(None);
    cache.write(1, 0, Bytes::from("AAA"), "test").await;
    cache.write(1, 3, Bytes::from("BBB"), "test").await;
    cache.write(1, 6, Bytes::from("CCC"), "test").await;
    let data = cache.read(1, 2, 5).await;
    assert_eq!(data, Some(Bytes::from("ABBCC")));
}

#[tokio::test]
async fn test_read_with_gap_returns_none() {
    let cache = make_cache(None);
    cache.write(1, 0, Bytes::from("hello"), "test").await;
    let data = cache.read(1, 10, 5).await;
    assert!(data.is_none());
}

#[tokio::test]
async fn test_read_different_file_id() {
    let cache = make_cache(None);
    cache.write(1, 0, Bytes::from("file1"), "test").await;
    cache.write(2, 0, Bytes::from("file2"), "test").await;
    let data = cache.read(2, 0, 5).await;
    assert_eq!(data, Some(Bytes::from("file2")));
}

#[tokio::test]
async fn test_backward_merge_combines_entries() {
    let cache = make_cache(None);
    cache.write(1, 0, Bytes::from("AAAA"), "test").await;
    cache.write(1, 4, Bytes::from("BBBB"), "test").await;
    let data = cache.read(1, 0, 8).await;
    assert_eq!(data, Some(Bytes::from("AAAABBBB")));
}

#[tokio::test]
async fn test_forward_merge_combines_entries() {
    let cache = make_cache(None);
    cache.write(1, 4, Bytes::from("BBBB"), "test").await;
    cache.write(1, 0, Bytes::from("AAAA"), "test").await;
    let data = cache.read(1, 0, 8).await;
    assert_eq!(data, Some(Bytes::from("AAAABBBB")));
}

#[tokio::test]
async fn test_merge_respects_1mb_limit() {
    let cache = make_cache(None);
    let big = Bytes::from(vec![b'X'; 600_000]);
    let big2 = Bytes::from(vec![b'Y'; 600_000]);
    cache.write(1, 0, big, "test").await;
    cache.write(1, 600_000, big2, "test").await;
    let data = cache.read(1, 0, 1_200_000).await;
    assert_eq!(data, Some(Bytes::from(vec![b'X'; 600_000].into_iter().chain(vec![b'Y'; 600_000]).collect::<Vec<_>>())));
}

#[tokio::test]
async fn test_eviction_removes_distant_entries() {
    let cache = make_cache(Some(50));
    cache.write(1, 0, Bytes::from("AAAAA"), "test").await;
    cache.write(1, 5, Bytes::from("BBBBB"), "test").await;
    cache.write(1, 10, Bytes::from("CCCCC"), "test").await;
    cache.write(1, 20, Bytes::from("DDDDD"), "test").await;
    let data = cache.read(1, 0, 5).await;
    assert!(data.is_some());
    let d = cache.read(1, 20, 5).await;
    assert_eq!(d, Some(Bytes::from("DDDDD")));
}

#[tokio::test]
async fn test_pinned_entries_not_evicted() {
    let cache = make_cache(Some(50));
    cache.write(1, 0, Bytes::from("AAAAA"), "test").await;
    cache.write(1, 5, Bytes::from("BBBBB"), "test").await;
    cache.set_pin_window(1, 2, 30, 100, "test").await;
    cache.write(1, 10, Bytes::from("CCCCC"), "test").await;
    cache.write(1, 20, Bytes::from("DDDDD"), "test").await;
    let a = cache.read(1, 0, 5).await;
    assert_eq!(a, Some(Bytes::from("AAAAA")));
}

#[tokio::test]
async fn test_wait_range_eventually_returns() {
    let cache = make_cache(None);
    let cache2 = cache.clone();
    tokio::spawn(async move {
        tokio::time::sleep(std::time::Duration::from_millis(20)).await;
        cache2.write(1, 0, Bytes::from("data"), "test").await;
    });
    let data = cache.wait_range(1, 0, 4).await;
    assert_eq!(data, Ok(Bytes::from("data")));
}

#[tokio::test]
async fn test_clear_removes_all() {
    let cache = make_cache(None);
    cache.write(1, 0, Bytes::from("hello"), "test").await;
    cache.clear().await;
    let data = cache.read(1, 0, 5).await;
    assert!(data.is_none());
}

#[tokio::test]
async fn test_multiple_partitions_isolated() {
    let mut cfg = HashMap::new();
    cfg.insert("a".into(), PartitionConfig { max_bytes: None });
    cfg.insert("b".into(), PartitionConfig { max_bytes: None });
    let cache = PartitionedMemCache::new(cfg);
    cache.write(1, 0, Bytes::from("aaaa"), "a").await;
    cache.write(1, 0, Bytes::from("bbbb"), "b").await;
    assert_eq!(cache.read(1, 0, 4).await, Some(Bytes::from("aaaa")));
}

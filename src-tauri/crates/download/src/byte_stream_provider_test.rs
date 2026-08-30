use std::collections::HashMap;

use omega_drive_gateway::provider::storage::PartMetadata;

use super::*;

fn test_part(part_index: u32, size: i64) -> PartMetadata {
    PartMetadata {
        id: i64::from(part_index),
        file_id: 1,
        platform: "telegram".to_string(),
        message_id: format!("msg-{part_index}"),
        part_index,
        size,
        checksum: None,
    }
}

fn make_parts_data(parts: &[(u32, i64)])
    -> (Vec<u32>, HashMap<u32, u64>, HashMap<u32, PartMetadata>)
{
    let all_parts: HashMap<u32, PartMetadata> = parts.iter()
        .map(|&(idx, sz)| (idx, test_part(idx, sz)))
        .collect();
    let mut sorted_idx: Vec<u32> = all_parts.keys().copied().collect();
    sorted_idx.sort_unstable();
    let mut part_starts = HashMap::with_capacity(sorted_idx.len());
    let mut cumul = 0u64;
    for &idx in &sorted_idx {
        part_starts.insert(idx, cumul);
        cumul += all_parts[&idx].size.max(0) as u64;
    }
    (sorted_idx, part_starts, all_parts)
}

#[test]
fn lower_bound_returns_first_part_for_offset_0() {
    let (sorted, starts, parts) = make_parts_data(&[(1, 100), (2, 100), (3, 80)]);
    let idx = lower_bound_part(&sorted, &starts, &parts, 0);
    assert_eq!(sorted[idx], 1);
}

#[test]
fn lower_bound_returns_first_part_for_mid_offset() {
    let (sorted, starts, parts) = make_parts_data(&[(1, 100), (2, 100), (3, 80)]);
    let idx = lower_bound_part(&sorted, &starts, &parts, 50);
    assert_eq!(sorted[idx], 1);
}

#[test]
fn lower_bound_returns_second_part_at_boundary() {
    let (sorted, starts, parts) = make_parts_data(&[(1, 100), (2, 100), (3, 80)]);
    let idx = lower_bound_part(&sorted, &starts, &parts, 100);
    assert_eq!(sorted[idx], 2);
}

#[test]
fn lower_bound_returns_last_part_for_beyond_file() {
    let (sorted, starts, parts) = make_parts_data(&[(1, 100), (2, 100), (3, 80)]);
    let idx = lower_bound_part(&sorted, &starts, &parts, 999);
    assert_eq!(sorted[idx], 3);
}

#[test]
fn lower_bound_returns_last_part_at_last_byte() {
    let (sorted, starts, parts) = make_parts_data(&[(1, 100), (2, 100), (3, 80)]);
    let idx = lower_bound_part(&sorted, &starts, &parts, 279);
    assert_eq!(sorted[idx], 3);
}

#[test]
fn lower_bound_with_single_part() {
    let (sorted, starts, parts) = make_parts_data(&[(5, 200)]);
    let idx = lower_bound_part(&sorted, &starts, &parts, 0);
    assert_eq!(sorted[idx], 5);
}

#[test]
fn lower_bound_with_two_parts_at_mid() {
    let (sorted, starts, parts) = make_parts_data(&[(1, 50), (2, 50)]);
    let idx = lower_bound_part(&sorted, &starts, &parts, 60);
    assert_eq!(sorted[idx], 2);
}

#[test]
fn lower_bound_and_loop_handles_gap_in_idx() {
    let (sorted, starts, parts) = make_parts_data(&[(1, 100), (5, 100), (9, 100)]);
    // sorted = [1,5,9], file offsets 0,100,200
    let idx = lower_bound_part(&sorted, &starts, &parts, 150);
    assert_eq!(sorted[idx], 5);
    let idx2 = lower_bound_part(&sorted, &starts, &parts, 250);
    assert_eq!(sorted[idx2], 9);
}

#[test]
fn sorted_idx_slice_handles_sparse_loop() {
    let sorted = vec![1u32, 5, 9];
    let first_idx = 0; // tương ứng part 1
    let last_idx = 2;  // tương ứng part 9
    let slice = &sorted[first_idx..=last_idx];
    assert_eq!(slice, &[1, 5, 9]);
    // đảm bảo không có 2,3,4,6,7,8
    assert!(!slice.contains(&2));
}

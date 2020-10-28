use super::{MergeNumberHash, NumberHash};
use crate::{leaf_index_to_mmr_size, util::MemStore, Error, MMR};
use faster_hex::hex_string;
use proptest::prelude::*;
use rand::{seq::SliceRandom, thread_rng};

fn test_mmr(count: u32, proof_elem: Vec<u32>) {
	test_mmr_inner(count, proof_elem, None, None)
}

fn test_mmr_pruning(count: u32, proof_elem: Vec<u32>, pruning_elem: u32) {
	test_mmr_inner(count, proof_elem, None, Some(pruning_elem))
}

fn test_mmr_and_proof_len(count: u32, proof_elem: Vec<u32>, proof_size: Option<usize>) {
	test_mmr_inner(count, proof_elem, proof_size, None)
}

fn test_mmr_inner(
	count: u32,
	proof_elem: Vec<u32>,
	proof_size: Option<usize>,
	pruning_elem: Option<u32>,
) {
    let store = MemStore::default();
    let mut mmr = MMR::<_, MergeNumberHash, _>::new(0, &store);
    let positions: Vec<u64> = (0u32..count)
        .map(|i| mmr.push(NumberHash::from(i)).unwrap())
        .collect();
		let mmr = if let Some(pruning) = pruning_elem {
			let len = mmr.mmr_size();
			mmr.commit().expect("commit changes");
			let position = leaf_index_to_mmr_size((pruning - 1) as u64);
			// pruning -> bad values, this is worst possible
			// more position - nb 1 at the end
			for i in 0..(position / 2) {
				store.inner().borrow_mut().remove(&i);
			}
			MMR::<_, MergeNumberHash, _>::new(len, &store)
		} else {
			mmr
		};
    let root = mmr.get_root().expect("get root");
    let proof = mmr
        .gen_proof(
            proof_elem
                .iter()
                .map(|elem| positions[*elem as usize])
                .collect(),
        )
        .expect("gen proof");
		if let Some(expected_size) = proof_size {
			assert_eq!(proof.proof_items().len(), expected_size);
		}
    mmr.commit().expect("commit changes");
    let result = proof
        .verify(
            root,
            proof_elem
                .iter()
                .map(|elem| (positions[*elem as usize], NumberHash::from(*elem)))
                .collect(),
        )
        .unwrap();
    assert!(result);
}

fn test_gen_new_root_from_proof(count: u32) {
    let store = MemStore::default();
    let mut mmr = MMR::<_, MergeNumberHash, _>::new(0, &store);
    let positions: Vec<u64> = (0u32..count)
        .map(|i| mmr.push(NumberHash::from(i)).unwrap())
        .collect();
    let elem = count - 1;
    let pos = positions[elem as usize];
    let proof = mmr.gen_proof(vec![pos]).expect("gen proof");
    let new_elem = count;
    let new_pos = mmr.push(NumberHash::from(new_elem)).unwrap();
    let root = mmr.get_root().expect("get root");
    mmr.commit().expect("commit changes");
    let calculated_root = proof
        .calculate_root_with_new_leaf(
            vec![(pos, NumberHash::from(elem))],
            new_pos,
            NumberHash::from(new_elem),
            leaf_index_to_mmr_size(new_elem.into()),
        )
        .unwrap();
    assert_eq!(calculated_root, root);
}

#[test]
fn test_mmr_root() {
    let store = MemStore::default();
    let mut mmr = MMR::<_, MergeNumberHash, _>::new(0, &store);
    (0u32..11).for_each(|i| {
        mmr.push(NumberHash::from(i)).unwrap();
    });
    let root = mmr.get_root().expect("get root");
    let hex_root = hex_string(&root.0).unwrap();
    assert_eq!(
        "dddd55a20b94975d197095dd00bc8077e8225b4e31f694dce3a7370d6c94fec5",
        hex_root
    );
}

#[test]
fn test_empty_mmr_root() {
    let store = MemStore::<NumberHash>::default();
    let mmr = MMR::<_, MergeNumberHash, _>::new(0, &store);
    assert_eq!(Err(Error::GetRootOnEmpty), mmr.get_root());
}

#[test]
fn test_mmr_3_peaks() {
    test_mmr(11, vec![5]);
}

#[test]
fn test_mmr_2_peaks() {
    test_mmr(10, vec![5]);
}

#[test]
fn test_mmr_1_peak() {
    test_mmr(8, vec![5]);
}

#[test]
fn test_mmr_first_elem_proof() {
    test_mmr(11, vec![0]);
}

#[test]
fn test_mmr_last_elem_proof() {
    test_mmr(11, vec![10]);
}

#[test]
fn test_mmr_1_elem() {
    test_mmr(1, vec![0]);
}

#[test]
fn test_mmr_2_elems() {
    test_mmr(2, vec![0]);
    test_mmr(2, vec![1]);
}

#[test]
fn test_mmr_2_leaves_merkle_proof() {
    test_mmr(11, vec![3, 7]);
    test_mmr(11, vec![3, 4]);
}

#[test]
fn test_mmr_2_sibling_leaves_merkle_proof() {
    test_mmr(11, vec![4, 5]);
    test_mmr(11, vec![5, 6]);
    test_mmr(11, vec![6, 7]);
}

#[test]
fn test_mmr_3_leaves_merkle_proof() {
    test_mmr(11, vec![4, 5, 6]);
    test_mmr(11, vec![3, 5, 7]);
    test_mmr(11, vec![3, 4, 5]);
    test_mmr(100, vec![3, 5, 13]);
}

#[test]
fn test_gen_root_from_proof() {
    test_gen_new_root_from_proof(11);
}

#[test]
fn test_mmr_proof_size() {
	// len 8 is single peak so a binary tree of depth 3
  test_mmr_and_proof_len(8, vec![0], Some(3));
  test_mmr_and_proof_len(8, vec![7], Some(3));
  test_mmr_and_proof_len(8, vec![0, 1], Some(2));
  test_mmr_and_proof_len(8, vec![0, 2], Some(3));
  test_mmr_and_proof_len(8, vec![0, 5, 6], Some(4));
  test_mmr_and_proof_len(8, vec![0, 7], Some(4));
	// 2 for peak root and one for baging
  test_mmr_and_proof_len(5, vec![0], Some(3));
  test_mmr_and_proof_len(6, vec![0], Some(3));
	// 2 for peak root and one for baging (bagging against merge of 2nd and 3rd peaks)
  test_mmr_and_proof_len(7, vec![0], Some(3));
	// 2 for peak one, one for peak two, and peak 3
  test_mmr_and_proof_len(7, vec![0, 4], Some(4));
	// 2 for peak one, one for peak two root
  test_mmr_and_proof_len(7, vec![0, 6], Some(3));
  test_mmr_and_proof_len(7, vec![0, 1], Some(2));
  test_mmr_and_proof_len(7, vec![3], Some(3));
	// 2nd peak one, 3rd peak and 1rst peak
  test_mmr_and_proof_len(7, vec![4], Some(3));
	// 3 peak and bag of peaks
  test_mmr_and_proof_len(15, vec![0], Some(4));
	// 2 2nd peak and bag 3rd 4th and first
  test_mmr_and_proof_len(15, vec![8], Some(4));
	// 1 3rd peak and 4th peak and 2nd peak and first peak
  test_mmr_and_proof_len(15, vec![12], Some(4));
	// 3 other peak
  test_mmr_and_proof_len(15, vec![14], Some(3));
	// comp first and second peak: 3 for 1st, 2 for 2nd, 1 bag
  test_mmr_and_proof_len(15, vec![0, 8], Some(6));
	// comp first and third peak: 3 for 1st, 1 for 3rd, 4th peak, 2nd peak
  test_mmr_and_proof_len(15, vec![0, 12], Some(6));
	// comp first and fourth peak: 3 for 1st, 2nd and 3rd peak
  test_mmr_and_proof_len(15, vec![0, 14], Some(5));
	// comp second and third peak: 2 for 2nd, 1 for thrid first and 4th peak
  test_mmr_and_proof_len(15, vec![8, 12], Some(5));
	// comp second and fouth peak: 2 for 2nd, first and 3rd peak
  test_mmr_and_proof_len(15, vec![8, 14], Some(4));
	// comp third and fourth peak: 1 for third and 1st and 2nd
  test_mmr_and_proof_len(15, vec![12, 14], Some(3));
}

#[test]
fn to_check() {
/*	test_mmr(2, vec![0]);
	test_mmr(2, vec![1]);
	test_mmr(3, vec![1]);
	test_mmr(7, vec![0]);
	test_mmr(7, vec![3]);*/
	test_mmr(3, vec![2]);
}


#[test]
fn test_pruning() {
	let size = [1, 7, 8, 15, 16];
	for s in size.iter() {
		for i in 1..*s {
		 test_mmr_pruning(*s, vec![i], i);
		}
	}
}

prop_compose! {
    fn count_elem(count: u32)
                (elem in 0..count)
                -> (u32, u32) {
                    (count, elem)
    }
}

proptest! {
    #[test]
    fn test_random_mmr(count in 10u32..500u32) {
        let mut leaves: Vec<u32> = (0..count).collect();
        let mut rng = thread_rng();
        leaves.shuffle(&mut rng);
        let leaves_count = rng.gen_range(1, count - 1);
        leaves.truncate(leaves_count as usize);
        test_mmr(count, leaves);
    }

    #[test]
    fn test_random_gen_root_with_new_leaf(count in 1u32..500u32) {
        test_gen_new_root_from_proof(count);
    }
}

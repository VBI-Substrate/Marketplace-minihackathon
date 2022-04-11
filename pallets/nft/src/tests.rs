use crate::{mock::*, Error};
use frame_support::{assert_noop, assert_ok};

#[test]
fn it_works_for_default_value() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		assert_ok!(PalletNFT::mint(
			1u64,
			[0u8; 16],
			Some(vec!(1)),
			Some(vec!(1)),
			Some(vec!(1)),
			Some(vec!(1)),
			Some(0),
			vec!((5, 1)),
			[0u8; 16]
		));
	});
}
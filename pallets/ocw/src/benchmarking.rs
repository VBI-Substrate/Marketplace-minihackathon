//! Benchmarking setup for pallet-template
#![cfg(feature = "runtime-benchmarks")]
use super::*;

#[allow(unused)]
use crate::Pallet as NFT;
use frame_benchmarking::{benchmarks, whitelisted_caller, account};
use frame_system::RawOrigin;
use frame_support::traits::Currency;
use sp_std::vec;
use sp_std::vec::Vec;

benchmarks! {
	submit_number_unsigned {
		let s in 0 .. 100;
		let caller: T::AccountId = whitelisted_caller();
	}: _(RawOrigin::Signed(caller), s)

	impl_benchmark_test_suite!(NFT, crate::mock::new_test_ext(), crate::mock::Test);
}

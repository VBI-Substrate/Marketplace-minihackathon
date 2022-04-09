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
	mint_nft {
		let s in 0 .. 100;
		let royalty_account: T::AccountId = account("royalty_account", 2u32, 2u32);
		let caller: T::AccountId = whitelisted_caller();
		let title: Option<Vec<u16>> = Some(vec!(s.clone().try_into().unwrap()));
		let description: Option<Vec<u128>> = Some(vec!(s.clone().into()));
		let media: Option<Vec<u128>> = Some(vec!(s.clone().into()));
		let media_hash: Option<Vec<u128>> = Some(vec!(s.clone().into()));
		let co_owner: Option<T::AccountId> = Some(royalty_account.clone());
		let royalty: Vec<(T::AccountId, u32)> = vec!((royalty_account.clone(), s.clone().into()));
	}: _(RawOrigin::Signed(caller), title, description, media, media_hash, co_owner, royalty)

	edit_nft {
		let s in 0 .. 100;
		let nft_id = [0u8; 16];
		let caller: T::AccountId = whitelisted_caller();
		let royalty_account: T::AccountId = account("royalty_account", 2u32, 2u32);
		let percent = 1;

		NFT::<T>::mint(
			caller,
			nft_id.clone(),
			Some(vec!(1)),
			Some(vec!(s.clone().into())),
			Some(vec!(s.clone().into())),
			Some(vec!(s.clone().into())),
			Some(royalty_account.clone()),
			vec!((royalty_account.clone(), percent.clone())),
		);

		let caller: T::AccountId = whitelisted_caller();
		let title: Option<Vec<u16>> = Some(vec!(1));
		let description: Option<Vec<u128>> = Some(vec!(s.clone().into()));
		let media: Option<Vec<u128>> = Some(vec!(s.clone().into()));
		let media_hash: Option<Vec<u128>> = Some(vec!(s.clone().into()));
		let co_owner: Option<T::AccountId> = Some(royalty_account.clone());
		let royalty: Vec<(T::AccountId, u32)> = vec!((royalty_account.clone(), percent.clone()));

	}: _(RawOrigin::Signed(caller), nft_id, title, description, media, media_hash, co_owner, royalty)

	buy_nft {
		let s in 0 .. 10000000;
		let caller: T::AccountId = whitelisted_caller();
		let owner: T::AccountId = account("owner", 2u32, 2u32);
		let royalty_account: T::AccountId = account("royalty_account", 2u32, 2u32);
		let percent = 1;

		NFT::<T>::mint(
			owner.clone(),
			[0u8; 16],
			Some(vec!(1)),
			Some(vec!(s.clone().into())),
			Some(vec!(s.clone().into())),
			Some(vec!(s.clone().into())),
			Some(royalty_account.clone()),
			vec!((royalty_account.clone(), percent.clone())),
		);

		let _ = NFT::<T>::set_sale_nft(RawOrigin::Signed(owner.clone()).into(), [0u8; 16], Some(s.into()));
		
		let balance = T::Currency::minimum_balance() * s.into();
        let _ = T::Currency::make_free_balance_be(&caller, balance);
	
	}: _(RawOrigin::Signed(caller), [0u8; 16])

	set_sale_nft{ 
		let s in 0 .. 10000000;
		let caller: T::AccountId = whitelisted_caller();
		let royalty_account: T::AccountId = account("royalty_account", 2u32, 2u32);
		let percent = 1;
		
		NFT::<T>::mint(
			caller.clone(),
			[0u8; 16],
			Some(vec!(1)),
			Some(vec!(s.clone().into())),
			Some(vec!(s.clone().into())),
			Some(vec!(s.clone().into())),
			Some(royalty_account.clone()),
			vec!((royalty_account.clone(), percent.clone())),
		);
	
	}: _(RawOrigin::Signed(caller), [0u8; 16], Some(s.into()))
	
	set_nft_price{ 
		let s in 0 .. 10000000;
		let caller: T::AccountId = whitelisted_caller();
		let royalty_account: T::AccountId = account("royalty_account", 2u32, 2u32);
		let percent = 1;

		NFT::<T>::mint(
			caller.clone(),
			[0u8; 16],
			Some(vec!(1)),
			Some(vec!(s.clone().into())),
			Some(vec!(s.clone().into())),
			Some(vec!(s.clone().into())),
			Some(royalty_account.clone()),
			vec!((royalty_account.clone(), percent.clone())),
		);

		NFT::<T>::set_sale_nft(RawOrigin::Signed(caller.clone()).into(), [0u8; 16], Some(s.into()));

	}: _(RawOrigin::Signed(caller), [0u8; 16], Some(s.into()))

	burn_nft {
		let s in 0 .. 10000000;
		let caller: T::AccountId = whitelisted_caller();
		let royalty_account: T::AccountId = account("royalty_account", 2u32, 2u32);
		let percent = 1;

		NFT::<T>::mint(
			caller.clone(),
			[0u8; 16],
			Some(vec!(1)),
			Some(vec!(s.clone().into())),
			Some(vec!(s.clone().into())),
			Some(vec!(s.clone().into())),
			Some(royalty_account.clone()),
			vec!((royalty_account.clone(), percent.clone())),
		);
		
	}: _(RawOrigin::Signed(caller), [0u8; 16])

	impl_benchmark_test_suite!(NFT, crate::mock::new_test_ext(), crate::mock::Test);
}

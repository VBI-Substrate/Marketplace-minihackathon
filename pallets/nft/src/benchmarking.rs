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
		let collection_id = [0u8; 16];
		let caller: T::AccountId = whitelisted_caller();

		NFT::<T>::mint_collection(
			caller.clone(),
			collection_id.clone(),
			vec!(1),
			Some(vec!(s.clone() as u8)),
		);

		let royalty_account: T::AccountId = account("royalty_account", 2u32, 2u32);
		let caller: T::AccountId = whitelisted_caller();
		let title: Vec<u8> = vec!(s.clone().try_into().unwrap());
		let description: Option<Vec<u8>> = Some(vec!(s.clone() as u8));
		let media: Vec<u8> = vec!(s.clone() as u8);
		let media_hash: Vec<u8> = vec!(s.clone() as u8);
		let installment_account: Option<T::AccountId> = Some(royalty_account.clone());
		let royalty: Option<Vec<(T::AccountId, u8)>> = Some(vec!((royalty_account.clone(), s.clone().try_into().unwrap())));
	
		let balance = T::Currency::minimum_balance() * NFT::<T>::u32_to_balance(1000000u32);
        let _ = T::Currency::make_free_balance_be(&caller, balance);
	
	}: _(RawOrigin::Signed(caller), title, description, media, media_hash, installment_account, royalty, collection_id)

	create_collection {
		let s in 0 .. 100;
		let caller: T::AccountId = whitelisted_caller();
		let title: Vec<u8> = vec!(s.clone().try_into().unwrap());
		let description: Option<Vec<u8>> = Some(vec!(s.clone() as u8));

		let balance = T::Currency::minimum_balance() * NFT::<T>::u32_to_balance(1000000u32);
        let _ = T::Currency::make_free_balance_be(&caller, balance);

	}: _(RawOrigin::Signed(caller), title, description)

	destroy_collection {
		let s in 0 .. 100;
		let collection_id = [0u8; 16];
		let caller: T::AccountId = whitelisted_caller();

		NFT::<T>::mint_collection(
			caller.clone(),
			collection_id.clone(),
			vec!(1),
			Some(vec!(s.clone() as u8)),
		);

	}: _(RawOrigin::Signed(caller), collection_id)

	edit_nft {
		let s in 0 .. 100;
		let nft_id = [0u8; 16];
		let caller: T::AccountId = whitelisted_caller();
		let royalty_account: T::AccountId = account("royalty_account", 2u32, 2u32);
		let percent = 1;

		NFT::<T>::mint_collection(
			caller.clone(),
			nft_id.clone(),
			vec!(1),
			Some(vec!(s.clone() as u8)),
		);

		NFT::<T>::mint(
			caller.clone(),
			nft_id.clone(),
			vec!(1),
			Some(vec!(s.clone() as u8)),
			vec!(s.clone() as u8),
			vec!(s.clone() as u8),
			Some(royalty_account.clone()),
			Some(vec!((royalty_account.clone(), percent.clone()))),
			nft_id.clone(),
		);

		let caller: T::AccountId = whitelisted_caller();
		let title: Vec<u8> = vec!(1);
		let description: Option<Vec<u8>> = Some(vec!(s.clone() as u8));
		let media: Vec<u8> = vec!(s.clone() as u8);
		let media_hash: Vec<u8> = vec!(s.clone() as u8);
		let installment_account: Option<T::AccountId> = Some(royalty_account.clone());
		let royalty: Option<Vec<(T::AccountId, u8)>> = Some(vec!((royalty_account.clone(), percent.clone().into())));

		let balance = T::Currency::minimum_balance() * NFT::<T>::u32_to_balance(1000000u32);
        let _ = T::Currency::make_free_balance_be(&caller, balance);

	}: _(RawOrigin::Signed(caller), nft_id, title, description, media, media_hash, installment_account, royalty, nft_id)

	edit_collection {
		let s in 0 .. 100;
		let collection_id = [0u8; 16];
		let caller: T::AccountId = whitelisted_caller();

		NFT::<T>::mint_collection(
			caller.clone(),
			collection_id.clone(),
			vec!(1),
			Some(vec!(s.clone() as u8)),
		);

		let title: Option<Vec<u8>> = Some(vec!(1));
		let description: Option<Vec<u8>> = Some(vec!(s.clone() as u8));

		let balance = T::Currency::minimum_balance() * NFT::<T>::u32_to_balance(1000000u32);
        let _ = T::Currency::make_free_balance_be(&caller, balance);

	}: _(RawOrigin::Signed(caller), collection_id, title, description)

	buy_nft {
		let s in 0 .. 10000000;
		let caller: T::AccountId = whitelisted_caller();
		let owner: T::AccountId = account("owner", 2u32, 2u32);
		let royalty_account: T::AccountId = account("royalty_account", 2u32, 2u32);
		let percent = 1;

		NFT::<T>::mint_collection(
			caller.clone(),
			[0u8; 16],
			vec!(1),
			Some(vec!(s.clone() as u8)),
		);

		NFT::<T>::mint(
			owner.clone(),
			[0u8; 16],
			vec!(1),
			Some(vec!(s.clone() as u8)),
			vec!(s.clone() as u8),
			vec!(s.clone() as u8),
			Some(royalty_account.clone()),
			Some(vec!((royalty_account.clone(), percent.clone()))),
			[0u8; 16]
		);

		let _ = NFT::<T>::set_sale_nft(RawOrigin::Signed(owner.clone()).into(), [0u8; 16], s.into());
		
		let balance = T::Currency::minimum_balance() * NFT::<T>::u32_to_balance(1000000u32);
        let _ = T::Currency::make_free_balance_be(&caller, balance);
		
	}: _(RawOrigin::Signed(caller), [0u8; 16])

	pay_installment{
		let s in 0 .. 10000000;
		let caller: T::AccountId = whitelisted_caller();
		let owner: T::AccountId = account("owner", 2u32, 2u32);
		let royalty_account: T::AccountId = account("royalty_account", 2u32, 2u32);
		let percent = 1;

		NFT::<T>::mint_collection(
			caller.clone(),
			[0u8; 16],
			vec!(1),
			Some(vec!(s.clone() as u8)),
		);

		NFT::<T>::mint(
			owner.clone(),
			[0u8; 16],
			vec!(1),
			Some(vec!(s.clone() as u8)),
			vec!(s.clone() as u8),
			vec!(s.clone() as u8),
			Some(royalty_account.clone()),
			Some(vec!((royalty_account.clone(), percent.clone()))),
			[0u8; 16]
		);

		let _ = NFT::<T>::set_sale_nft(RawOrigin::Signed(owner.clone()).into(), [0u8; 16], s.into());
		
		let balance = T::Currency::minimum_balance() * NFT::<T>::u32_to_balance(1000000u32);
        let _ = T::Currency::make_free_balance_be(&caller, balance);
	
	}: _(RawOrigin::Signed(caller), [0u8; 16], 5, s.into())

	set_sale_nft{ 
		let s in 0 .. 10000000;
		let caller: T::AccountId = whitelisted_caller();
		let royalty_account: T::AccountId = account("royalty_account", 2u32, 2u32);
		let percent = 1;

		NFT::<T>::mint_collection(
			caller.clone(),
			[0u8; 16],
			vec!(1),
			Some(vec!(s.clone() as u8)),
		);
		
		NFT::<T>::mint(
			caller.clone(),
			[0u8; 16],
			vec!(1),
			Some(vec!(s.clone() as u8)),
			vec!(s.clone() as u8),
			vec!(s.clone() as u8),
			Some(royalty_account.clone()),
			Some(vec!((royalty_account.clone(), percent.clone()))),
			[0u8; 16]
		);
	
		let balance = T::Currency::minimum_balance() * NFT::<T>::u32_to_balance(1000000u32);
        let _ = T::Currency::make_free_balance_be(&caller, balance);

	}: _(RawOrigin::Signed(caller), [0u8; 16], s.into())
	
	set_nft_price{ 
		let s in 0 .. 10000000;
		let caller: T::AccountId = whitelisted_caller();
		let royalty_account: T::AccountId = account("royalty_account", 2u32, 2u32);
		let percent = 1;

		NFT::<T>::mint_collection(
			caller.clone(),
			[0u8; 16],
			vec!(1),
			Some(vec!(s.clone() as u8)),
		);

		NFT::<T>::mint(
			caller.clone(),
			[0u8; 16],
			vec!(1),
			Some(vec!(s.clone() as u8)),
			vec!(s.clone() as u8),
			vec!(s.clone() as u8),
			Some(royalty_account.clone()),
			Some(vec!((royalty_account.clone(), percent.clone()))),
			[0u8; 16]
		);

		let balance = T::Currency::minimum_balance() * NFT::<T>::u32_to_balance(1000000u32);
        let _ = T::Currency::make_free_balance_be(&caller, balance);

		NFT::<T>::set_sale_nft(RawOrigin::Signed(caller.clone()).into(), [0u8; 16], s.into());

	}: _(RawOrigin::Signed(caller), [0u8; 16], s.into())

	burn_nft {
		let s in 0 .. 10000000;
		let caller: T::AccountId = whitelisted_caller();
		let royalty_account: T::AccountId = account("royalty_account", 2u32, 2u32);
		let percent = 1;

		NFT::<T>::mint_collection(
			caller.clone(),
			[0u8; 16],
			vec!(1),
			Some(vec!(s.clone() as u8)),
		);

		NFT::<T>::mint(
			caller.clone(),
			[0u8; 16],
			vec!(1),
			Some(vec!(s.clone() as u8)),
			vec!(s.clone() as u8),
			vec!(s.clone() as u8),
			Some(royalty_account.clone()),
			Some(vec!((royalty_account.clone(), percent.clone()))),
			[0u8; 16]
		);
		
		let balance = T::Currency::minimum_balance() * NFT::<T>::u32_to_balance(1000000u32);
        let _ = T::Currency::make_free_balance_be(&caller, balance);

	}: _(RawOrigin::Signed(caller), [0u8; 16])

	impl_benchmark_test_suite!(NFT, crate::mock::new_test_ext(), crate::mock::Test);
}

#![cfg_attr(not(feature = "std"), no_std)]

pub use pallet::*;

// #[cfg(test)]
// mod mock;

// #[cfg(test)]
// mod tests;

#[cfg(feature = "runtime-benchmarks")]
pub mod benchmarking;
pub mod weights;

#[frame_support::pallet]
pub mod pallet {
	use frame_support::{
		pallet_prelude::*,
		traits::{tokens::ExistenceRequirement, Currency, Randomness},
		transactional, require_transactional
	};
	use frame_system::pallet_prelude::*;
	use scale_info::TypeInfo;
	use sp_io::hashing::blake2_128;
	use sp_std::vec::Vec;
    use pallet_nft;
	use crate::weights::WeightInfo;

	#[cfg(feature = "std")]
	use frame_support::serde::{Deserialize, Serialize};

	type BalanceOf<T> =
		<<T as Config>::CurrencyMarketplace as Currency<<T as frame_system::Config>::AccountId>>::Balance;

	#[derive(Clone, Encode, Decode, PartialEq, RuntimeDebug, TypeInfo)]
	#[scale_info(skip_type_params(T))]
	pub struct Sale<Account, Balance>{
		pub owner: Option<Account>,
		pub price: Option<Balance>,
		pub in_installment: Option<bool>
	}

	#[pallet::pallet]
	#[pallet::generate_store(pub(super) trait Store)]
	#[pallet::without_storage_info]
	pub struct Pallet<T>(_);

	// Configure the pallet by specifying the parameters and types on which it depends.
	#[pallet::config]
	pub trait Config: frame_system::Config + pallet_nft::Config {
		/// Because this pallet emits events, it depends on the runtime's definition of an event.
		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;

		type CurrencyMarketplace: Currency<Self::AccountId>;

		type NFTRandomnessMarketplace: Randomness<Self::Hash, Self::BlockNumber>;

		type WeightInfoMarketplace: WeightInfo;
	}

	// Errors
	#[pallet::error]
	pub enum Error<T> {
		NoNFT,
		NoCollection,
		NotOwner,
		DuplicateNFT,
		DuplicateCollection,
		NFTInInstallment,
		TransferToSelf,
		NotForSale,
		NotSelling,
		NFTOnSale,
		BurntNFT
	}

	// Events
	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		PriceSet { nft: [u8; 16], price: Option<BalanceOf<T>> },
		SetSaleNFT { nft: [u8; 16], price: Option<BalanceOf<T>> },
		NFTOnSale { nft: [u8; 16], price: Option<BalanceOf<T>> },
		BurntNFT { nft: [u8; 16] },
		Bought { seller: T::AccountId, buyer: T::AccountId, nft: [u8; 16], price: BalanceOf<T> },
		Transferred { from: T::AccountId, to: T::AccountId, nft: [u8; 16] },
	}

	#[pallet::storage]
	#[pallet::getter(fn token_sale)]
	pub(super) type TokenSale<T: Config> = StorageMap<_, Twox64Concat, [u8; 16], Sale<T::AccountId, BalanceOf<T>>>;

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		#[pallet::weight(<T as Config>::WeightInfoMarketplace::buy_nft())]
		#[transactional]
		pub fn buy_nft(
			origin: OriginFor<T>,
			nft_id: [u8; 16]
		) -> DispatchResult {
			// Make sure the caller is from a signed origin
			let buyer = ensure_signed(origin)?;

			Self::do_transfer(nft_id, buyer)?;

			Ok(())
		}

		#[pallet::weight(<T as Config>::WeightInfoMarketplace::set_sale_nft())]
		#[transactional]
		pub fn set_sale_nft(
			origin: OriginFor<T>,
			nft_id: [u8; 16],
			new_price: Option<BalanceOf<T>>,
		) -> DispatchResult {
			// Make sure the caller is from a signed origin
			let sender = ensure_signed(origin)?;

			let nft = pallet_nft::Pallet::<T>::token_by_id(&nft_id).ok_or(Error::<T>::NoNFT)?;
			ensure!(nft.is_burnt == Some(false), Error::<T>::BurntNFT);

			let nft_on_sale = TokenSale::<T>::get(&nft_id);
			match nft_on_sale {
				Some(nft_on_sale) => {
					Self::deposit_event(Event::NFTOnSale { nft: nft_id, price: nft_on_sale.price });
				},
				None => {
					let token_sale = Sale::<T::AccountId, BalanceOf<T>> {
						owner: Some(sender),
						price: new_price,
						in_installment: Some(false)
					};
					// Set the price in storage
					TokenSale::<T>::insert(&nft_id, token_sale);
					Self::deposit_event(Event::SetSaleNFT { nft: nft_id, price: new_price });
				}
			}
			Ok(())
		}

		#[pallet::weight(<T as Config>::WeightInfoMarketplace::set_nft_price())]
		#[transactional]
		pub fn set_nft_price(
			origin: OriginFor<T>,
			nft_id: [u8; 16],
			new_price: Option<BalanceOf<T>>,
		) -> DispatchResult {
			// Make sure the caller is from a signed origin
			let sender = ensure_signed(origin)?;

			// Ensure the kitty exists and is called by the kitty owner
			let nft = pallet_nft::Pallet::<T>::token_by_id(&nft_id).ok_or(Error::<T>::NoNFT)?;
			ensure!(nft.owner == Some(sender), Error::<T>::NotOwner);
			ensure!(nft.is_burnt == Some(false), Error::<T>::BurntNFT);

			let mut nft_on_sale = TokenSale::<T>::get(&nft_id).ok_or(Error::<T>::NotSelling)?;
			ensure!(nft_on_sale.in_installment == Some(false), Error::<T>::NFTInInstallment);

			nft_on_sale.price = new_price;
			TokenSale::<T>::insert(&nft_id, nft_on_sale);

			// Deposit a "PriceSet" event.
			Self::deposit_event(Event::PriceSet { nft: nft_id, price: new_price });

			Ok(())
		}
	}

	//** Our helper functions.**//

	impl<T: Config> Pallet<T> {
		pub fn gen_id() -> [u8; 16] {
			// Create randomness
			let random = T::NFTRandomness::random(&b"id"[..]).0;

			// Create randomness payload. Multiple kitties can be generated in the same block,
			// retaining uniqueness.
			let unique_payload = (
				random,
				frame_system::Pallet::<T>::extrinsic_index().unwrap_or_default(),
				frame_system::Pallet::<T>::block_number(),
			);

			// Turns into a byte array
			let encoded_payload = unique_payload.encode();
			let hash = blake2_128(&encoded_payload);

			hash
		}

		#[require_transactional]
		pub fn do_transfer(
			nft_id: [u8; 16],
			to: T::AccountId,
		) -> DispatchResult {
			let mut nft = pallet_nft::Pallet::<T>::token_by_id(&nft_id).ok_or(Error::<T>::NoNFT)?;
			let from = nft.owner;

			let mut nft_on_sale = TokenSale::<T>::get(&nft_id).ok_or(Error::<T>::NoNFT)?;
			ensure!(nft_on_sale.price != None, Error::<T>::NotSelling);
			ensure!(from != Some(to.clone()), Error::<T>::TransferToSelf);
			ensure!(nft.is_burnt == Some(false), Error::<T>::BurntNFT);
			
			let old_owner = from.unwrap();
			let new_owner = to;

			let royalty = nft.royalty.clone();
			let mut total_perpetual:BalanceOf<T> = 0u32.into();
			if royalty.len()>0 {
				for (k, percent) in royalty.iter() {
					let key = k.clone();
					if key != old_owner.clone() {
						let percent_type_balance:BalanceOf<T> = Self::u32_to_balance(*percent);
						let per_perpetual = percent_type_balance*nft_on_sale.price.unwrap();
						total_perpetual += per_perpetual;
						T::CurrencyMarketplace::transfer(&new_owner, &key, per_perpetual, ExistenceRequirement::KeepAlive)?;
					}
				}
			}
			let after_price = nft_on_sale.price.unwrap()-total_perpetual;
			// Transfer the amount from buyer to seller
			T::CurrencyMarketplace::transfer(&new_owner, &old_owner, after_price, ExistenceRequirement::KeepAlive)?;
			// Deposit sold event
			Self::deposit_event(Event::Bought {
				seller: old_owner.clone(),
				buyer: new_owner.clone(),
				nft: nft_id,
				price: after_price,
			});

			// Transfer succeeded, update the kitty owner and reset the price to `None`.
			let default_price:BalanceOf<T> = 0u32.into();
			nft.owner = Some(new_owner.clone());
			nft_on_sale.owner = Some(new_owner.clone());
			nft_on_sale.price = Some(default_price);

            // Write updates to storage
			pallet_nft::Pallet::<T>::insert_to_token_by_id(&nft_id, nft);
			TokenSale::<T>::insert(&nft_id, nft_on_sale);

			Self::deposit_event(Event::Transferred { from: old_owner, to: new_owner.clone(), nft: nft_id });

			Ok(())
		}

		pub fn u32_to_balance(input: u32) -> BalanceOf<T> {
			input.into()
		}
	}
}
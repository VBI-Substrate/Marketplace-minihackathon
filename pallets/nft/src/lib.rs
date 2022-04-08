#![cfg_attr(not(feature = "std"), no_std)]

pub use pallet::*;

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

#[frame_support::pallet]
pub mod pallet {
	use frame_support::{
		pallet_prelude::*,
		traits::{tokens::ExistenceRequirement, Currency, Randomness}
	};
	use frame_system::pallet_prelude::*;
	use scale_info::TypeInfo;
	use sp_io::hashing::blake2_128;
	use sp_std::vec::Vec;

	#[cfg(feature = "std")]
	use frame_support::serde::{Deserialize, Serialize};

	// Handles our pallet's currency abstraction
	type BalanceOf<T> =
		<<T as Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;

	// Struct for holding kitty information
	#[derive(Clone, Encode, Decode, PartialEq, RuntimeDebug, TypeInfo)]
	#[scale_info(skip_type_params(T))]
	pub struct NonFungibleToken<T: Config> {
		pub title: Option<Vec<u16>>, // ex. "Arch Nemesis: Mail Carrier" or "Parcel #5055"
		pub description: Option<Vec<u128>>, // free-form description
		pub media: Option<Vec<u128>>, // URL to associated media, preferably to decentralized, content-addressed storage
		pub media_hash: Option<Vec<u128>>, // Base64-encoded sha256 hash of content referenced by the `media` field. Required if `media` is included.
		pub owner: Option<T::AccountId>,
		pub co_owner: Option<T::AccountId>, // paying installment
		pub royalty: Vec<(T::AccountId, u32)>,
		pub is_burnt: Option<bool>
	}

	#[derive(Clone, Encode, Decode, PartialEq, RuntimeDebug, TypeInfo)]
	#[scale_info(skip_type_params(T))]
	pub struct Sale<T: Config>{
		pub owner: Option<T::AccountId>,
		pub price: Option<BalanceOf<T>>,
		pub in_installment: Option<bool>
	}

	impl<T: Config> MaxEncodedLen for NonFungibleToken<T> {
        fn max_encoded_len() -> usize {
            T::AccountId::max_encoded_len() * 2
        }
    }

	impl<T: Config> MaxEncodedLen for Sale<T> {
        fn max_encoded_len() -> usize {
            T::AccountId::max_encoded_len() * 2
        }
    }

	#[pallet::pallet]
	#[pallet::generate_store(pub(super) trait Store)]
	pub struct Pallet<T>(_);

	// Configure the pallet by specifying the parameters and types on which it depends.
	#[pallet::config]
	pub trait Config: frame_system::Config {
		/// Because this pallet emits events, it depends on the runtime's definition of an event.
		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;

		/// The Currency handler for the kitties pallet.
		type Currency: Currency<Self::AccountId>;

		/// The type of Randomness we want to specify for this pallet.
		type NFTRandomness: Randomness<Self::Hash, Self::BlockNumber>;
	}

	// Errors
	#[pallet::error]
	pub enum Error<T> {
		NoNFT,
		NotOwner,
		DuplicateNFT,
		NFTInInstallment,
		TransferToSelf,
		NotForSale,
		NotSelling,
		NFTOnSale
	}

	// Events
	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		Created { nft: [u8; 16], owner: T::AccountId },
		PriceSet { nft: [u8; 16], price: Option<BalanceOf<T>> },
		SetSaleNFT { nft: [u8; 16], price: Option<BalanceOf<T>> },
		NFTOnSale { nft: [u8; 16], price: Option<BalanceOf<T>> },
		BurntNFT { nft: [u8; 16] },
		Bought { seller: T::AccountId, buyer: T::AccountId, nft: [u8; 16], price: BalanceOf<T> },
		Transferred { from: T::AccountId, to: T::AccountId, nft: [u8; 16] },
	}

	/// Maps the kitty struct to the kitty DNA.
	#[pallet::storage]
	#[pallet::getter(fn token_by_id)]
	pub(super) type TokenById<T: Config> = StorageMap<_, Twox64Concat, [u8; 16], NonFungibleToken<T>>;

	#[pallet::storage]
	#[pallet::getter(fn token_sale)]
	pub(super) type TokenSale<T: Config> = StorageMap<_, Twox64Concat, [u8; 16], Sale<T>>;

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		#[pallet::weight(0)]
		pub fn mint_nft(
			origin: OriginFor<T>,
			title: Option<Vec<u16>>,
			description: Option<Vec<u128>>,
			media: Option<Vec<u128>>,
			media_hash: Option<Vec<u128>>,
			co_owner: Option<T::AccountId>,
			royalty: Vec<(T::AccountId, u32)>
		) -> DispatchResult {
			// Make sure the caller is from a signed origin
			let sender = ensure_signed(origin)?;
			// Generate unique DNA and Gender using a helper function
			let nft_id = Self::gen_id();

			// Write new kitty to storage by calling helper function
			let nft = NonFungibleToken::<T> { 
				title,
				description,
				media,
				media_hash,
				owner: Some(sender.clone()),
				co_owner,
				royalty,
				is_burnt: Some(false)
			};
			
			// Check if the kitty does not already exist in our storage map
			ensure!(!TokenById::<T>::contains_key(&nft_id), Error::<T>::DuplicateNFT);

			// Write new kitty to storage
			TokenById::<T>::insert(nft_id, nft);

			// Deposit our "Created" event.
			Self::deposit_event(Event::Created { nft: nft_id, owner: sender });

			// Returns the DNA of the new kitty if this succeeds
			Ok(())
		}

		#[pallet::weight(0)]
		pub fn buy_nft(
			origin: OriginFor<T>,
			nft_id: [u8; 16]
		) -> DispatchResult {
			// Make sure the caller is from a signed origin
			let buyer = ensure_signed(origin)?;
			// Transfer the kitty from seller to buyer as a sale.
			Self::do_transfer(nft_id, buyer)?;

			Ok(())
		}

		#[pallet::weight(0)]
		pub fn burn_nft(
			origin: OriginFor<T>,
			nft_id: [u8; 16]
		) -> DispatchResult {
			// Make sure the caller is from a signed origin
			let sender = ensure_signed(origin)?;

			let mut nft = TokenById::<T>::get(&nft_id).ok_or(Error::<T>::NoNFT)?;
			ensure!(nft.owner == Some(sender), Error::<T>::NotOwner);

			// Set the price in storage
			nft.is_burnt = Some(true);
			TokenById::<T>::insert(&nft_id, nft);

			Self::deposit_event(Event::BurntNFT { nft: nft_id });

			Ok(())
		}

		#[pallet::weight(0)]
		pub fn set_sale_nft(
			origin: OriginFor<T>,
			nft_id: [u8; 16],
			new_price: Option<BalanceOf<T>>,
		) -> DispatchResult {
			// Make sure the caller is from a signed origin
			let sender = ensure_signed(origin)?;

			// Ensure the kitty exists and is called by the kitty owner
			let nft_on_sale = TokenSale::<T>::get(&nft_id);
			match nft_on_sale {
				Some(nft_on_sale) => {
					Self::deposit_event(Event::NFTOnSale { nft: nft_id, price: nft_on_sale.price });
				},
				None => {
					let token_sale = Sale::<T> {
						owner: Some(sender),
						price: new_price,
						in_installment: Some(false)
					};
					// Set the price in storage
					TokenSale::<T>::insert(&nft_id, token_sale);
					Self::deposit_event(Event::SetSaleNFT { nft: nft_id, price: new_price });
				}
			}
			// Deposit a "PriceSet" event.

			Ok(())
		}

		#[pallet::weight(0)]
		pub fn set_nft_price(
			origin: OriginFor<T>,
			nft_id: [u8; 16],
			new_price: Option<BalanceOf<T>>,
		) -> DispatchResult {
			// Make sure the caller is from a signed origin
			let sender = ensure_signed(origin)?;

			// Ensure the kitty exists and is called by the kitty owner
			let nft = TokenById::<T>::get(&nft_id).ok_or(Error::<T>::NoNFT)?;
			ensure!(nft.owner == Some(sender), Error::<T>::NotOwner);
			ensure!(nft.is_burnt == Some(false), Error::<T>::NFTInInstallment);

			let nft_on_sale = TokenSale::<T>::get(&nft_id);
			match nft_on_sale {
				Some(nft_on_sale) => {
					let mut tmp_nft = nft_on_sale;
					tmp_nft.price = new_price;
					TokenSale::<T>::insert(&nft_id, tmp_nft);
				},
				None => {
					let token_sale = Sale::<T> {
						owner: nft.owner,
						price: new_price,
						in_installment: Some(false)
					};
					TokenSale::<T>::insert(&nft_id, token_sale);
				}
			}

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

		pub fn do_transfer(
			nft_id: [u8; 16],
			to: T::AccountId,
		) -> DispatchResult {
			let mut nft = TokenById::<T>::get(&nft_id).ok_or(Error::<T>::NoNFT)?;
			let from = nft.owner;

			let mut nft_on_sale = TokenSale::<T>::get(&nft_id).ok_or(Error::<T>::NoNFT)?;
			ensure!(nft_on_sale.price != None, Error::<T>::NotSelling);
			ensure!(from != Some(to.clone()), Error::<T>::TransferToSelf);
			
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
						T::Currency::transfer(&new_owner, &key, per_perpetual, ExistenceRequirement::KeepAlive)?;
					}
				}
			}
			let after_price = nft_on_sale.price.unwrap()-total_perpetual;
			// Transfer the amount from buyer to seller
			T::Currency::transfer(&new_owner, &old_owner, after_price, ExistenceRequirement::KeepAlive)?;
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
			TokenById::<T>::insert(&nft_id, nft);
			TokenSale::<T>::insert(&nft_id, nft_on_sale);

			Self::deposit_event(Event::Transferred { from: old_owner, to: new_owner.clone(), nft: nft_id });

			Ok(())
		}

		pub fn u32_to_balance(input: u32) -> BalanceOf<T> {
			input.into()
		}

		pub fn mint(
			sender: T::AccountId,
			nft_id: [u8; 16],
			title: Option<Vec<u16>>,
			description: Option<Vec<u128>>,
			media: Option<Vec<u128>>,
			media_hash: Option<Vec<u128>>,
			co_owner: Option<T::AccountId>,
			royalty: Vec<(T::AccountId, u32)>
		) -> DispatchResult {
			// Write new kitty to storage by calling helper function
			let nft = NonFungibleToken::<T> { 
				title,
				description,
				media,
				media_hash,
				owner: Some(sender.clone()),
				co_owner,
				royalty,
				is_burnt: Some(false)
			};
			
			// Check if the kitty does not already exist in our storage map
			ensure!(!TokenById::<T>::contains_key(&nft_id), Error::<T>::DuplicateNFT);

			// Write new kitty to storage
			TokenById::<T>::insert(nft_id, nft);

			// Deposit our "Created" event.
			Self::deposit_event(Event::Created { nft: nft_id, owner: sender });

			// Returns the DNA of the new kitty if this succeeds
			Ok(())
		}
	}
}
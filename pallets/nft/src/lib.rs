#![cfg_attr(not(feature = "std"), no_std)]

pub use pallet::*;

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

#[cfg(feature = "runtime-benchmarks")]
pub mod benchmarking;
pub mod weights;

#[frame_support::pallet]
pub mod pallet {
	use frame_support::{
		pallet_prelude::*,
		sp_runtime::traits::{Scale},
		traits::{tokens::ExistenceRequirement, Currency, Randomness, ReservableCurrency, Time},
		transactional, require_transactional
	};
	use frame_system::pallet_prelude::*;
	use scale_info::{TypeInfo, StaticTypeInfo};
	use sp_io::hashing::blake2_128;
	use sp_std::vec::Vec;
	use crate::weights::WeightInfo;
	use frame_support::sp_runtime::traits::Saturating;
	use frame_support::traits::Len;

	#[cfg(feature = "std")]
	use frame_support::serde::{Deserialize, Serialize};

	type BalanceOf<T> =
		<<T as Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;

	#[derive(Clone, Encode, Decode, PartialEq, RuntimeDebug, TypeInfo)]
	#[scale_info(skip_type_params(T))]
	pub struct NFTCollection<Account, Balance> {
		pub title: Option<Vec<u8>>,
		pub description: Option<Vec<u8>>,
		pub creator: Option<Account>,
		pub deposit: Option<Balance>
	}

	#[derive(Clone, Encode, Decode, PartialEq, RuntimeDebug, TypeInfo)]
	#[scale_info(skip_type_params(T))]
	pub struct NonFungibleToken<Account, Balance> {
		pub title: Option<Vec<u8>>, // ex. "Arch Nemesis: Mail Carrier" or "Parcel #5055"
		pub description: Option<Vec<u8>>, // free-form description
		pub media: Option<Vec<u8>>, // URL to associated media, preferably to decentralized, content-addressed storage
		pub media_hash: Option<Vec<u8>>, // Base64-encoded sha256 hash of content referenced by the `media` field. Required if `media` is included.
		pub creator: Option<Account>,
		pub owner: Option<Account>,
		pub installment_account: Option<Account>, // paying installment
		pub royalty: Vec<(Account, u8)>,
		pub collection_id: [u8; 16],
		pub deposit: Option<Balance>
	}

	#[derive(Clone, Encode, Decode, PartialEq, RuntimeDebug, TypeInfo)]
	#[scale_info(skip_type_params(T))]
	pub struct PayInstallmentOrder<Account, Balance, Time> {
		pub creator: Account,
		pub pay_at: Time,
        pub periods_left: u8,
		pub paid: Balance
	}

	#[derive(Clone, Encode, Decode, PartialEq, RuntimeDebug, TypeInfo)]
	#[scale_info(skip_type_params(T))]
	pub struct Sale<Account, Balance>{
		pub owner: Option<Account>,
		pub price: Option<Balance>,
		pub in_installment: Option<bool>,
		pub deposit: Option<Balance>
	}
	#[pallet::pallet]
	#[pallet::generate_store(pub(super) trait Store)]
	#[pallet::without_storage_info]
	pub struct Pallet<T>(_);

	// Configure the pallet by specifying the parameters and types on which it depends.
	#[pallet::config]
	pub trait Config: frame_system::Config {
		/// Because this pallet emits events, it depends on the runtime's definition of an event.
		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;

		type Currency: ReservableCurrency<Self::AccountId> + Currency<Self::AccountId>;

		type NFTRandomness: Randomness<Self::Hash, Self::BlockNumber>;

		type WeightInfo: WeightInfo;

		/// Deposit required for per byte.
		#[pallet::constant]
		type DataDepositPerByte: Get<BalanceOf<Self>>;

		type Moment: Parameter
			+ Default
			+ Scale<Self::BlockNumber, Output = Self::Moment>
			+ Copy
			+ MaxEncodedLen
			+ StaticTypeInfo
			+ MaybeSerializeDeserialize
			+ Send;

		type Timestamp: Time<Moment = Self::Moment>;
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
		BurntNFT,
		TokenInCollection,
		NoOrder,
		FromOneToSixMonths,
		PriceNotMatch,
	}

	// Events
	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		Created { nft: [u8; 16], owner: T::AccountId },
		CreatedCollection { collection: [u8; 16], owner: T::AccountId },
		Edited { nft: [u8; 16], owner: T::AccountId },
		EditedCollection { collection: [u8; 16], owner: T::AccountId },
		PriceSet { nft: [u8; 16], price: Option<BalanceOf<T>> },
		SetSaleNFT { nft: [u8; 16], price: Option<BalanceOf<T>> },
		NFTOnSale { nft: [u8; 16], price: Option<BalanceOf<T>> },
		BurntNFT { nft: [u8; 16] },
		DestroyCollection { collection: [u8; 16] },
		Bought { seller: T::AccountId, buyer: T::AccountId, nft: [u8; 16], price: BalanceOf<T> },
		Transferred { from: T::AccountId, to: T::AccountId, nft: [u8; 16] },
		Paid { nft_id: [u8; 16], periods_left: u8 },
	}

	#[pallet::storage]
	#[pallet::getter(fn collection_by_id)]
	pub(super) type CollectionById<T: Config> = StorageMap<_, Twox64Concat, [u8; 16], NFTCollection<T::AccountId, BalanceOf<T>>>;

	#[pallet::storage]
	#[pallet::getter(fn token_by_id)]
	pub(super) type TokenById<T: Config> = StorageMap<_, Twox64Concat, [u8; 16], NonFungibleToken<T::AccountId, BalanceOf<T>>>;

	#[pallet::storage]
	#[pallet::getter(fn token_sale)]
	pub(super) type TokenSale<T: Config> = StorageMap<_, Twox64Concat, [u8; 16], Sale<T::AccountId, BalanceOf<T>>>;

	#[pallet::storage]
	#[pallet::getter(fn order_by_id)]
	pub(super) type OrderByTokenId<T: Config> = StorageMap<_, Twox64Concat, [u8; 16], PayInstallmentOrder<T::AccountId, BalanceOf<T>, T::Moment>>;
	
	#[pallet::call]
	impl<T: Config> Pallet<T> {
		#[pallet::weight(<T as Config>::WeightInfo::mint_nft())]
		#[transactional]
		pub fn mint_nft(
			origin: OriginFor<T>,
			title: Option<Vec<u8>>,
			description: Option<Vec<u8>>,
			media: Option<Vec<u8>>,
			media_hash: Option<Vec<u8>>,
			installment_account: Option<T::AccountId>,
			royalty: Vec<(T::AccountId, u8)>,
			collection_id: [u8; 16]
		) -> DispatchResult {
			// Make sure the caller is from a signed origin
			let sender = ensure_signed(origin)?;

			let nft_id = Self::gen_id();

			let data_compressed:u8 = u8::try_from(title.len()).unwrap().saturating_add(u8::try_from(description.len()).unwrap()).saturating_add(u8::try_from(media.len()).unwrap()).saturating_add(u8::try_from(media_hash.len()).unwrap()).saturating_add(u8::try_from(royalty.len()).unwrap().saturating_mul(16)).saturating_add(u8::try_from(collection_id.len()).unwrap()).saturating_add(32);
			let data_deposit = T::DataDepositPerByte::get().saturating_mul(data_compressed.into());
			
			T::Currency::reserve(&sender, data_deposit.clone())?;
			
			let nft = NonFungibleToken::<T::AccountId, BalanceOf<T>> { 
				title,
				description,
				media,
				media_hash,
				creator: Some(sender.clone()),
				owner: Some(sender.clone()),
				installment_account,
				royalty,
				collection_id,
				deposit: Some(data_deposit)
			};
			
			ensure!(!TokenById::<T>::contains_key(&nft_id), Error::<T>::DuplicateNFT);

			TokenById::<T>::insert(nft_id, nft);

			// Deposit our "Created" event.
			Self::deposit_event(Event::Created { nft: nft_id, owner: sender });

			Ok(())
		}

		#[pallet::weight(<T as Config>::WeightInfo::create_collection())]
		#[transactional]
		pub fn create_collection(
			origin: OriginFor<T>,
			title: Option<Vec<u8>>,
			description: Option<Vec<u8>>,
		) -> DispatchResult {
			// Make sure the caller is from a signed origin
			let sender = ensure_signed(origin)?;

			let collection_id = Self::gen_id();

			let data_compressed:u32 = u32::try_from(title.len()).unwrap().saturating_add(u32::try_from(description.len()).unwrap()).saturating_add(16);
			let data_deposit = T::DataDepositPerByte::get().saturating_mul(data_compressed.into());

			T::Currency::reserve(&sender, data_deposit.clone())?;
			
			let collection = NFTCollection::<T::AccountId, BalanceOf<T>> { 
				title,
				description,
				creator: Some(sender.clone()),
				deposit: Some(data_deposit)
			};
			
			ensure!(!CollectionById::<T>::contains_key(&collection_id), Error::<T>::DuplicateCollection);

			CollectionById::<T>::insert(collection_id, collection);

			// Deposit our "Created" event.
			Self::deposit_event(Event::CreatedCollection { collection: collection_id, owner: sender });

			Ok(())
		}

		#[pallet::weight(<T as Config>::WeightInfo::create_collection())]
		#[transactional]
		pub fn destroy_collection(
			origin: OriginFor<T>,
			collection_id: [u8; 16],
		) -> DispatchResult {
			// Make sure the caller is from a signed origin
			let sender = ensure_signed(origin)?;

			ensure!(CollectionById::<T>::contains_key(&collection_id), Error::<T>::NoCollection);
			let collection = CollectionById::<T>::get(collection_id.clone()).unwrap();
			ensure!(collection.creator.unwrap() == sender.clone(), Error::<T>::NotOwner);

			let mut check = 0;
			for nft in TokenById::<T>::iter_values() {
				if nft.collection_id == collection_id {
					check += 1;
					break;
				}
			}

			ensure!(check == 0, Error::<T>::TokenInCollection);

			T::Currency::unreserve(&sender, collection.deposit.unwrap());

			CollectionById::<T>::remove(collection_id.clone());

			// Deposit our "Created" event.
			Self::deposit_event(Event::DestroyCollection { collection: collection_id });

			Ok(())
		}

		#[pallet::weight(<T as Config>::WeightInfo::edit_nft())]
		#[transactional]
		pub fn edit_nft(
			origin: OriginFor<T>,
			nft_id: [u8; 16],
			title: Option<Vec<u8>>,
			description: Option<Vec<u8>>,
			media: Option<Vec<u8>>,
			media_hash: Option<Vec<u8>>,
			installment_account: Option<T::AccountId>,
			royalty: Vec<(T::AccountId, u8)>,
			collection_id: [u8; 16]
		) -> DispatchResult {
			// Make sure the caller is from a signed origin
			let sender = ensure_signed(origin)?;

			let mut nft = TokenById::<T>::get(&nft_id).ok_or(Error::<T>::NoNFT)?;
			ensure!(nft.owner == Some(sender.clone()), Error::<T>::NotOwner);

			let data_compressed:u32 = u32::try_from(title.len()).unwrap().saturating_add(u32::try_from(description.len()).unwrap()).saturating_add(u32::try_from(media.len()).unwrap()).saturating_add(u32::try_from(media_hash.len()).unwrap()).saturating_add(u32::try_from(royalty.len()).unwrap().saturating_mul(16)).saturating_add(u32::try_from(collection_id.len()).unwrap()).saturating_add(32);
			let data_deposit = T::DataDepositPerByte::get().saturating_mul(data_compressed.into());

			T::Currency::reserve(&sender, data_deposit.clone())?;

			nft.title = title;
			nft.description = description;
			nft.media = media;
			nft.media_hash = media_hash;
			nft.installment_account = installment_account;
			nft.royalty = royalty;
			nft.collection_id = collection_id;
			nft.deposit = Some(data_deposit);

			TokenById::<T>::insert(nft_id, nft);

			// Deposit our "Created" event.
			Self::deposit_event(Event::Edited { nft: nft_id, owner: sender });

			Ok(())
		}

		#[pallet::weight(<T as Config>::WeightInfo::edit_collection())]
		#[transactional]
		pub fn edit_collection(
			origin: OriginFor<T>,
			collection_id: [u8; 16],
			title: Option<Vec<u8>>,
			description: Option<Vec<u8>>,
		) -> DispatchResult {
			// Make sure the caller is from a signed origin
			let sender = ensure_signed(origin)?;

			let mut collection = CollectionById::<T>::get(&collection_id).ok_or(Error::<T>::NoCollection)?;
			ensure!(collection.creator == Some(sender.clone()), Error::<T>::NotOwner);

			let data_compressed:u32 = u32::try_from(title.len()).unwrap().saturating_add(u32::try_from(description.len()).unwrap()).saturating_add(16);
			let data_deposit = T::DataDepositPerByte::get().saturating_mul(data_compressed.into());

			T::Currency::reserve(&sender, data_deposit.clone())?;
			
			collection.title = title;
			collection.description = description;
			collection.deposit = Some(data_deposit);
			
			CollectionById::<T>::insert(collection_id, collection);

			// Deposit our "Created" event.
			Self::deposit_event(Event::EditedCollection { collection: collection_id, owner: sender });

			Ok(())
		}

		#[pallet::weight(<T as Config>::WeightInfo::buy_nft())]
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

		#[pallet::weight(0)]
		#[transactional]
		pub fn pay_installment(
			origin: OriginFor<T>,
			nft_id: [u8; 16],
			periods: u8,
			paid: BalanceOf<T>
		) -> DispatchResult {
			// Make sure the caller is from a signed origin
			let sender = ensure_signed(origin)?;

			ensure!(periods > 0, Error::<T>::FromOneToSixMonths);
			ensure!(periods < 7, Error::<T>::FromOneToSixMonths);

			let nft_on_sale = TokenSale::<T>::get(&nft_id).ok_or(Error::<T>::NotSelling)?;

			let order = OrderByTokenId::<T>::get(&nft_id);
			let periods_left = periods.clone() - 1;
			let mut redundant:BalanceOf<T> = Self::u8_to_balance(0u8);
			match order {
				Some(mut order) => {
					if periods_left == 0 && order.paid >= nft_on_sale.price.unwrap() {
						redundant = order.paid.saturating_sub(nft_on_sale.price.unwrap());
						OrderByTokenId::<T>::remove(&nft_id);
						// transfer
						TokenSale::<T>::remove(&nft_id);
						let mut nft = TokenById::<T>::get(&nft_id).ok_or(Error::<T>::NoNFT)?;
						nft.owner = Some(sender.clone());
						TokenById::<T>::insert(&nft_id, nft);
					} else {
						order.periods_left = periods_left;
						order.paid = order.paid.saturating_add(paid);
						OrderByTokenId::<T>::insert(&nft_id, order);
					}
				},
				None => {
					let order = PayInstallmentOrder::<T::AccountId, BalanceOf<T>, T::Moment> {
						creator: sender.clone(),
						pay_at: T::Timestamp::now(),
						paid,
						periods_left
					};
					OrderByTokenId::<T>::insert(&nft_id, order);
				}
			}

			T::Currency::transfer(&sender, &nft_on_sale.owner.unwrap(), paid.saturating_sub(redundant), ExistenceRequirement::KeepAlive)?;

			Self::deposit_event(Event::Paid { nft_id, periods_left });

			Ok(())
		}

		#[pallet::weight(<T as Config>::WeightInfo::burn_nft())]
		#[transactional]
		pub fn burn_nft(
			origin: OriginFor<T>,
			nft_id: [u8; 16]
		) -> DispatchResult {
			// Make sure the caller is from a signed origin
			let sender = ensure_signed(origin)?;

			let nft = TokenById::<T>::get(&nft_id).ok_or(Error::<T>::NoNFT)?;
			ensure!(nft.owner == Some(sender.clone()), Error::<T>::NotOwner);

			if let Some(nft_on_sale) = TokenSale::<T>::get(&nft_id) {
				ensure!(nft_on_sale.in_installment == Some(false), Error::<T>::NFTInInstallment);
			}

			T::Currency::unreserve(&sender, nft.deposit.unwrap());

			// Set the price in storage
			TokenById::<T>::remove(&nft_id);

			Self::deposit_event(Event::BurntNFT { nft: nft_id });

			Ok(())
		}

		#[pallet::weight(<T as Config>::WeightInfo::set_sale_nft())]
		#[transactional]
		pub fn set_sale_nft(
			origin: OriginFor<T>,
			nft_id: [u8; 16],
			new_price: BalanceOf<T>,
		) -> DispatchResult {
			// Make sure the caller is from a signed origin
			let sender = ensure_signed(origin)?;

			TokenById::<T>::get(&nft_id).ok_or(Error::<T>::NoNFT)?;

			let nft_on_sale = TokenSale::<T>::get(&nft_id);
			match nft_on_sale {
				Some(nft_on_sale) => {
					Self::deposit_event(Event::NFTOnSale { nft: nft_id, price: nft_on_sale.price });
				},
				None => {
					let data_compressed:u32 = 18;
					let data_deposit = T::DataDepositPerByte::get().saturating_mul(data_compressed.into());
					
					T::Currency::reserve(&sender, data_deposit.clone())?;

					let token_sale = Sale::<T::AccountId, BalanceOf<T>> {
						owner: Some(sender),
						price: Some(new_price.clone()),
						in_installment: Some(false),
						deposit: Some(data_deposit)
					};
					// Set the price in storage
					TokenSale::<T>::insert(&nft_id, token_sale);
					Self::deposit_event(Event::SetSaleNFT { nft: nft_id, price: Some(new_price) });
				}
			}
			Ok(())
		}

		#[pallet::weight(<T as Config>::WeightInfo::set_nft_price())]
		#[transactional]
		pub fn set_nft_price(
			origin: OriginFor<T>,
			nft_id: [u8; 16],
			new_price: BalanceOf<T>,
		) -> DispatchResult {
			// Make sure the caller is from a signed origin
			let sender = ensure_signed(origin)?;

			// Ensure the kitty exists and is called by the kitty owner
			let nft = TokenById::<T>::get(&nft_id).ok_or(Error::<T>::NoNFT)?;
			ensure!(nft.owner == Some(sender), Error::<T>::NotOwner);

			let mut nft_on_sale = TokenSale::<T>::get(&nft_id).ok_or(Error::<T>::NotSelling)?;
			ensure!(nft_on_sale.in_installment == Some(false), Error::<T>::NFTInInstallment);

			nft_on_sale.price = Some(new_price.clone());
			TokenSale::<T>::insert(&nft_id, nft_on_sale);

			// Deposit a "PriceSet" event.
			Self::deposit_event(Event::PriceSet { nft: nft_id, price: Some(new_price) });

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
						let percent_type_balance:BalanceOf<T> = Self::u8_to_balance(*percent);
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
			nft_on_sale.price = Some(default_price.clone());
			
			T::Currency::unreserve(&old_owner, nft.deposit.unwrap().saturating_add(nft_on_sale.deposit.unwrap()));
			
			nft.deposit = Some(default_price.clone());
			// Write updates to storage
			TokenById::<T>::insert(&nft_id, nft);
			TokenSale::<T>::remove(&nft_id);

			Self::deposit_event(Event::Transferred { from: old_owner, to: new_owner.clone(), nft: nft_id });

			Ok(())
		}

		pub fn u8_to_balance(input: u8) -> BalanceOf<T> {
			input.into()
		}

		pub fn balance_to_u8(input: BalanceOf<T>) -> u8 {
			TryInto::<u8>::try_into(input).ok().unwrap()
		}

		// For test and benchmark more quickly
		pub fn mint(
			sender: T::AccountId,
			nft_id: [u8; 16],
			title: Option<Vec<u8>>,
			description: Option<Vec<u8>>,
			media: Option<Vec<u8>>,
			media_hash: Option<Vec<u8>>,
			installment_account: Option<T::AccountId>,
			royalty: Vec<(T::AccountId, u8)>,
			collection_id: [u8; 16],
		) -> DispatchResult {
			let default_deposit = Self::u8_to_balance(0u8);

			let nft = NonFungibleToken::<T::AccountId, BalanceOf<T>> { 
				title,
				description,
				media,
				media_hash,
				creator: Some(sender.clone()),
				owner: Some(sender.clone()),
				installment_account,
				royalty,
				collection_id,
				deposit: Some(default_deposit)
			};
			
			ensure!(!TokenById::<T>::contains_key(&nft_id), Error::<T>::DuplicateNFT);

			TokenById::<T>::insert(nft_id, nft);

			// Deposit our "Created" event.
			Self::deposit_event(Event::Created { nft: nft_id, owner: sender });

			Ok(())
		}

		pub fn mint_collection(
			sender: T::AccountId,
			collection_id: [u8; 16],
			title: Option<Vec<u8>>,
			description: Option<Vec<u8>>,
		) -> DispatchResult {
			let default_deposit = Self::u8_to_balance(0u8);

			let collection = NFTCollection::<T::AccountId, BalanceOf<T>> { 
				title,
				description,
				creator: Some(sender.clone()),
				deposit: Some(default_deposit)
			};
			
			ensure!(!CollectionById::<T>::contains_key(&collection_id), Error::<T>::DuplicateCollection);

			CollectionById::<T>::insert(collection_id, collection);

			// Deposit our "Created" event.
			Self::deposit_event(Event::CreatedCollection { collection: collection_id, owner: sender });

			Ok(())
		}
	}
}
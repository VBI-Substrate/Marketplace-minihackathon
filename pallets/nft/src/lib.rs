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
		traits::{tokens::ExistenceRequirement, Currency, Randomness},
		transactional, require_transactional
	};
	use frame_system::pallet_prelude::*;
	use scale_info::TypeInfo;
	use sp_io::hashing::blake2_128;
	use sp_std::vec::Vec;
	use crate::weights::WeightInfo;

	#[cfg(feature = "std")]
	use frame_support::serde::{Deserialize, Serialize};

	type BalanceOf<T> =
		<<T as Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;


	#[derive(Clone, Encode, Decode, PartialEq, RuntimeDebug, TypeInfo)]
	#[scale_info(skip_type_params(T))]
	pub struct NFTCollection<Account> {
		pub title: Option<Vec<u16>>,
		pub description: Option<Vec<u128>>,
		pub creator: Option<Account>,
	}

	#[derive(Clone, Encode, Decode, PartialEq, RuntimeDebug, TypeInfo)]
	#[scale_info(skip_type_params(T))]
	pub struct NonFungibleToken<Account> {
		pub title: Option<Vec<u16>>, // ex. "Arch Nemesis: Mail Carrier" or "Parcel #5055"
		pub description: Option<Vec<u128>>, // free-form description
		pub media: Option<Vec<u128>>, // URL to associated media, preferably to decentralized, content-addressed storage
		pub media_hash: Option<Vec<u128>>, // Base64-encoded sha256 hash of content referenced by the `media` field. Required if `media` is included.
		pub creator: Option<Account>,
		pub owner: Option<Account>,
		pub installment_account: Option<Account>, // paying installment
		pub royalty: Vec<(Account, u32)>,
		pub is_burnt: Option<bool>,
		pub collection_id: [u8; 16]
	}

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
	pub trait Config: frame_system::Config {
		/// Because this pallet emits events, it depends on the runtime's definition of an event.
		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;

		type Currency: Currency<Self::AccountId>;

		type NFTRandomness: Randomness<Self::Hash, Self::BlockNumber>;

		type WeightInfo: WeightInfo;
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
		Created { nft: [u8; 16], owner: T::AccountId },
		CreatedCollection { collection: [u8; 16], owner: T::AccountId },
		Edited { nft: [u8; 16], owner: T::AccountId },
		EditedCollection { collection: [u8; 16], owner: T::AccountId },
		PriceSet { nft: [u8; 16], price: Option<BalanceOf<T>> },
		SetSaleNFT { nft: [u8; 16], price: Option<BalanceOf<T>> },
		NFTOnSale { nft: [u8; 16], price: Option<BalanceOf<T>> },
		BurntNFT { nft: [u8; 16] },
		Bought { seller: T::AccountId, buyer: T::AccountId, nft: [u8; 16], price: BalanceOf<T> },
		Transferred { from: T::AccountId, to: T::AccountId, nft: [u8; 16] },
	}

	#[pallet::storage]
	#[pallet::getter(fn collection_by_id)]
	pub(super) type CollectionById<T: Config> = StorageMap<_, Twox64Concat, [u8; 16], NFTCollection<T::AccountId>>;

	#[pallet::storage]
	#[pallet::getter(fn token_by_id)]
	pub(super) type TokenById<T: Config> = StorageMap<_, Twox64Concat, [u8; 16], NonFungibleToken<T::AccountId>>;

	#[pallet::storage]
	#[pallet::getter(fn token_sale)]
	pub(super) type TokenSale<T: Config> = StorageMap<_, Twox64Concat, [u8; 16], Sale<T::AccountId, BalanceOf<T>>>;

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		#[pallet::weight(<T as Config>::WeightInfo::mint_nft())]
		#[transactional]
		pub fn mint_nft(
			origin: OriginFor<T>,
			title: Option<Vec<u16>>,
			description: Option<Vec<u128>>,
			media: Option<Vec<u128>>,
			media_hash: Option<Vec<u128>>,
			installment_account: Option<T::AccountId>,
			royalty: Vec<(T::AccountId, u32)>,
			collection_id: [u8; 16]
		) -> DispatchResult {
			// Make sure the caller is from a signed origin
			let sender = ensure_signed(origin)?;

			let nft_id = Self::gen_id();

			let nft = NonFungibleToken::<T::AccountId> { 
				title,
				description,
				media,
				media_hash,
				creator: Some(sender.clone()),
				owner: Some(sender.clone()),
				installment_account,
				royalty,
				is_burnt: Some(false),
				collection_id
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
			title: Option<Vec<u16>>,
			description: Option<Vec<u128>>,
		) -> DispatchResult {
			// Make sure the caller is from a signed origin
			let sender = ensure_signed(origin)?;

			let collection_id = Self::gen_id();

			let collection = NFTCollection::<T::AccountId> { 
				title,
				description,
				creator: Some(sender.clone()),
			};
			
			ensure!(!CollectionById::<T>::contains_key(&collection_id), Error::<T>::DuplicateCollection);

			CollectionById::<T>::insert(collection_id, collection);

			// Deposit our "Created" event.
			Self::deposit_event(Event::CreatedCollection { collection: collection_id, owner: sender });

			Ok(())
		}

		#[pallet::weight(<T as Config>::WeightInfo::edit_nft())]
		#[transactional]
		pub fn edit_nft(
			origin: OriginFor<T>,
			nft_id: [u8; 16],
			title: Option<Vec<u16>>,
			description: Option<Vec<u128>>,
			media: Option<Vec<u128>>,
			media_hash: Option<Vec<u128>>,
			installment_account: Option<T::AccountId>,
			royalty: Vec<(T::AccountId, u32)>,
			collection_id: [u8; 16]
		) -> DispatchResult {
			// Make sure the caller is from a signed origin
			let sender = ensure_signed(origin)?;

			let mut nft = TokenById::<T>::get(&nft_id).ok_or(Error::<T>::NoNFT)?;
			ensure!(nft.owner == Some(sender.clone()), Error::<T>::NotOwner);

			nft.title = title;
			nft.description = description;
			nft.media = media;
			nft.media_hash = media_hash;
			nft.installment_account = installment_account;
			nft.royalty = royalty;
			nft.is_burnt = Some(false);
			nft.collection_id = collection_id;

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
			title: Option<Vec<u16>>,
			description: Option<Vec<u128>>,
		) -> DispatchResult {
			// Make sure the caller is from a signed origin
			let sender = ensure_signed(origin)?;

			let mut collection = CollectionById::<T>::get(&collection_id).ok_or(Error::<T>::NoCollection)?;
			ensure!(collection.creator == Some(sender.clone()), Error::<T>::NotOwner);

			collection.title = title;
			collection.description = description;
			
			CollectionById::<T>::insert(collection_id, collection);

			// Deposit our "Created" event.
			Self::deposit_event(Event::EditedCollection { collection: collection_id, owner: sender });

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

			let mut nft = TokenById::<T>::get(&nft_id).ok_or(Error::<T>::NoNFT)?;
			ensure!(nft.owner == Some(sender), Error::<T>::NotOwner);

			if let Some(nft_on_sale) = TokenSale::<T>::get(&nft_id) {
				ensure!(nft_on_sale.in_installment == Some(false), Error::<T>::NFTInInstallment);
			}

			// Set the price in storage
			nft.is_burnt = Some(true);
			TokenById::<T>::insert(&nft_id, nft);

			Self::deposit_event(Event::BurntNFT { nft: nft_id });

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

		pub fn u32_to_balance(input: u32) -> BalanceOf<T> {
			input.into()
		}

		pub fn insert_to_token_by_id(nft_id: &[u8; 16], nft: NonFungibleToken<T::AccountId>) {
			TokenById::<T>::insert(&nft_id, nft);
		}

		// For test and benchmark more quickly
		pub fn mint(
			sender: T::AccountId,
			nft_id: [u8; 16],
			title: Option<Vec<u16>>,
			description: Option<Vec<u128>>,
			media: Option<Vec<u128>>,
			media_hash: Option<Vec<u128>>,
			installment_account: Option<T::AccountId>,
			royalty: Vec<(T::AccountId, u32)>,
			collection_id: [u8; 16],
		) -> DispatchResult {
			let nft = NonFungibleToken::<T::AccountId> { 
				title,
				description,
				media,
				media_hash,
				creator: Some(sender.clone()),
				owner: Some(sender.clone()),
				installment_account,
				royalty,
				is_burnt: Some(false),
				collection_id,
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
			title: Option<Vec<u16>>,
			description: Option<Vec<u128>>,
		) -> DispatchResult {
			let collection = NFTCollection::<T::AccountId> { 
				title,
				description,
				creator: Some(sender.clone()),
			};
			
			ensure!(!CollectionById::<T>::contains_key(&collection_id), Error::<T>::DuplicateCollection);

			CollectionById::<T>::insert(collection_id, collection);

			// Deposit our "Created" event.
			Self::deposit_event(Event::CreatedCollection { collection: collection_id, owner: sender });

			Ok(())
		}
	}
}